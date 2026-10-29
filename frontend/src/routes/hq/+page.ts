import type { HexMap } from '#lib/map.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
	const map: HexMap = await fetch('/api/map', { cache: 'no-cache' }).then((r) => r.json());
	return { map };
};
