import { error, redirect } from '@sveltejs/kit';
import { load as fetchMe } from '#lib/me.svelte.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, url }) => {
	const response = await fetch('/api/session', { cache: 'no-cache' });
	if (response.status === 401) redirect(307, '/login');
	if (!response.ok) error(response.status, "Couldn't check your login");
	const me = await fetchMe(fetch);
	if (!me) redirect(307, '/login');
	if (me.stage === 'hq' || me.stage === 'done' || (me.stage === 'guild' && !url.searchParams.has('passport'))) redirect(307, '/hq');
	return { me };
};
