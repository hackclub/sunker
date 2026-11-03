import { error, redirect } from '@sveltejs/kit';
import { load as fetchMe } from '#lib/me.svelte.js';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch }) => {
	const response = await fetch('/api/session', { cache: 'no-cache' });
	if (response.status === 401) redirect(307, '/login');
	if (!response.ok) error(response.status, "Couldn't check your login");
	const me = await fetchMe(fetch);
	if (!me) redirect(307, '/login');
	if (!me.stage || me.stage === 'passport') redirect(307, '/onboarding');
};
