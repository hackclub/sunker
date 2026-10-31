import { redirect } from '@sveltejs/kit';
import { allowedTabs, TABS, type Access } from '#lib/admin/access.js';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch, url }) => {
	const response = await fetch('/api/admin', { cache: 'no-cache' });
	if (!response.ok) redirect(307, '/');
	const admin: Access = await response.json();
	const allowed = allowedTabs(admin);
	const tab = TABS.find((tab) => url.pathname.startsWith(`/admin/${tab.slug}`));
	if (tab && !admin.permissions[tab.slug]) redirect(307, allowed.length ? `/admin/${allowed[0].slug}` : '/');
	return { admin };
};
