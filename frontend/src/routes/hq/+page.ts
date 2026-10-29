import { error } from '@sveltejs/kit';
import type { HexMap } from '#lib/map.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
	const response = await fetch('/api/map', { cache: 'no-cache' });
	if (!response.ok) error(response.status, "Couldn't load the map");
	const map: HexMap = await response.json();
	return { map };
};
