export class Paged<T extends { id: number }> {
	items: T[] = $state([]);
	more = $state(true);
	loading = $state(true);
	error = $state('');
	#generation = 0;
	#timer: ReturnType<typeof setTimeout> | undefined;
	#path: string;
	#key: string;
	#noun: string;
	#query: () => URLSearchParams;

	constructor(path: string, key: string, noun: string, query: () => URLSearchParams) {
		this.#path = path;
		this.#key = key;
		this.#noun = noun;
		this.#query = query;
	}

	async load(reset: boolean) {
		if (this.loading && !reset) return;
		const current = ++this.#generation;
		this.loading = true;
		const params = this.#query();
		if (!reset && this.items.length) params.set('before', String(this.items[this.items.length - 1].id));
		let data: { more: boolean; [key: string]: unknown } | undefined;
		let status = '';
		try {
			const response = await fetch(`${this.#path}?${params}`, { cache: 'no-cache' });
			if (response.ok) data = await response.json();
			else status = ` (${response.status})`;
		} catch {}
		if (current !== this.#generation) return;
		this.loading = false;
		if (!data) {
			this.error = `Couldn't load ${this.#noun}${status}`;
			return;
		}
		this.error = '';
		const page = data[this.#key] as T[];
		this.items = reset ? page : [...this.items, ...page];
		this.more = data.more;
	}

	reload() {
		clearTimeout(this.#timer);
		this.#timer = setTimeout(() => this.load(true), 200);
		return () => clearTimeout(this.#timer);
	}

	watch(sentinel: HTMLElement) {
		const observer = new IntersectionObserver((entries) => {
			if (entries[0].isIntersecting && this.more && this.items.length) this.load(false);
		});
		observer.observe(sentinel);
		return () => observer.disconnect();
	}
}
