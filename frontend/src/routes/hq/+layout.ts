import { error, redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch }) => {
	const response = await fetch('/api/session', { cache: 'no-cache' });
	if (response.status === 401) redirect(307, '/login');
	if (!response.ok) error(response.status, "Couldn't check your login");
};
