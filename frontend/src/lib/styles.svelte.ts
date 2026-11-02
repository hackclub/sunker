type Manifest = { images: Record<string, { id: number; width: number; height: number }>; colours: Record<string, string> };

export const styles = $state<Manifest & { revision: number }>({ images: {}, colours: {}, revision: 0 });

let current = '';
let channel: BroadcastChannel | undefined;

export async function refresh(fetcher: typeof fetch = fetch) {
	const response = await fetcher('/api/styling', { cache: 'no-cache' });
	if (!response.ok) return;
	const text = await response.text();
	if (text === current) return;
	current = text;
	const manifest: Manifest = JSON.parse(text);
	styles.images = manifest.images;
	styles.colours = manifest.colours;
	styles.revision++;
}

function listen() {
	const changed = () => {
		channel?.postMessage(null);
		refresh().catch(() => {});
	};
	const source = new EventSource('/api/styling/events');
	source.onopen = changed;
	source.onmessage = changed;
	return new Promise<never>(() => {});
}

export async function watch(fetcher: typeof fetch) {
	await refresh(fetcher).catch(() => {});
	if (channel) return;
	channel = new BroadcastChannel('styling');
	channel.onmessage = () => refresh().catch(() => {});
	if (navigator.locks) navigator.locks.request('styling-events', listen);
	else listen();
}

export const image = (slug: string) => {
	const entry = styles.images[slug];
	return `/api/styling/images/${slug}${entry ? `?v=${entry.id}` : ''}`;
};
