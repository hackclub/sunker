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
	{ slug: 'administration', name: 'Administration' },
	{ slug: 'review', name: 'Review' },
	{ slug: 'moderation', name: 'Moderation' }
] as const;

export const ACTIONS = [
	{ slug: 'ban', name: 'Ban / delete' },
	{ slug: 'resources', name: 'Resources' },
	{ slug: 'chats', name: 'Chats / guild' }
] as const;

export const allowedTabs = (access: Access) => TABS.filter((tab) => access.permissions[tab.slug]);
