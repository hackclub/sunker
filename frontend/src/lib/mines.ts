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

export type Prize = { id: number; ring: number; name: string; tagline: string; description: string };

export type StockedPrize = Prize & { base_cost_cents: number; additional_cost_cents: number; fulfillment_links: string[] };
