export type Contract = {
	slug: string;
	ring: number;
	name: string;
	description: string;
	image_url: string | null;
	video_url: string | null;
	criteria: string[];
	resources: Record<string, number>;
};

export type Prize = { id: number; ring: number; name: string; tagline: string; description: string; image: string | null };

export type StockedPrize = Prize & { base_cost_cents: number; additional_cost_cents: number; fulfillment_links: string[]; image_url: string | null };

export type Ring = { ring: number; min_hours: number };

export const catalog = $state<{ contracts: Contract[]; prizes: Prize[]; rings: Ring[] } & { loaded: boolean; broken: boolean }>({ contracts: [], prizes: [], rings: [], loaded: false, broken: false });

export async function browse() {
	try {
		const response = await fetch('/api/contracts', { cache: 'no-cache' });
		if (!response.ok) throw new Error();
		Object.assign(catalog, await response.json(), { loaded: true, broken: false });
	} catch {
		catalog.broken = true;
	}
}
