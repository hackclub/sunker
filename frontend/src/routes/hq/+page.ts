import { error } from '@sveltejs/kit';
import type { HexMap, Territory } from '#lib/map.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
	const [response, held] = await Promise.all([fetch('/api/map', { cache: 'no-cache' }), fetch('/api/territory')]);
	if (!response.ok) error(response.status, "Couldn't load the map");
	const map: HexMap = await response.json();
	const territory: Territory[] = held.ok ? await held.json() : [];
	return { map, territory };
};
