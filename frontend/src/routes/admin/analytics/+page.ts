import { error } from '@sveltejs/kit';
import type { Analytics } from '#lib/admin/analytics.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('admin:analytics');
	const response = await fetch('/api/admin/analytics', { cache: 'no-cache' });
	if (!response.ok) error(response.status, "Couldn't load analytics");
	const analytics: Analytics = await response.json();
	return { analytics };
};
