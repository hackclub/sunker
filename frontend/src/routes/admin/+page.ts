import { redirect } from '@sveltejs/kit';
import { allowedTabs } from '#lib/admin/access.js';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ parent }) => {
	const { admin } = await parent();
	const allowed = allowedTabs(admin);
	redirect(307, allowed.length ? `/admin/${allowed[0].slug}` : '/');
};
