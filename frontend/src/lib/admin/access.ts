export type Permissions = {
	analytics: boolean;
	users: boolean;
	ban: boolean;
	resources: boolean;
	chats: boolean;
	moderation: boolean;
	administration: boolean;
	review: boolean;
};

export type Access = { superadmin: boolean; permissions: Permissions };

export const TABS = [
	{ slug: 'analytics', name: 'Analytics' },
	{ slug: 'users', name: 'Users' },
	{ slug: 'review', name: 'Review' }
] as const;

export const allowedTabs = (access: Access) => TABS.filter((tab) => access.permissions[tab.slug]);
