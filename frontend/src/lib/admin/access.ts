export type Permissions = {
	analytics: boolean;
	users: boolean;
	ban: boolean;
	resources: boolean;
	chats: boolean;
	moderation: boolean;
	administration: boolean;
	review: boolean;
	joel: boolean;
	mines: boolean;
	video: boolean;
	styling: boolean;
};

export type Access = { superadmin: boolean; permissions: Permissions };

export const TABS = [
	{ slug: 'analytics', name: 'Analytics' },
	{ slug: 'users', name: 'Users' },
	{ slug: 'administration', name: 'Administration' },
	{ slug: 'review', name: 'Review' },
	{ slug: 'moderation', name: 'Moderation' },
	{ slug: 'joel', name: 'Joel View' },
	{ slug: 'mines', name: 'Mines' },
	{ slug: 'video', name: 'Video Verification' },
	{ slug: 'styling', name: 'Styling' }
] as const;

export const ACTIONS = [
	{ slug: 'ban', name: 'Ban / delete' },
	{ slug: 'resources', name: 'Resources' },
	{ slug: 'chats', name: 'Chats / guild' }
] as const;

export const NAV: { slug: string; name: string; permission: keyof Permissions }[] = [
	...TABS.slice(0, 2).map((tab) => ({ ...tab, permission: tab.slug })),
	{ slug: 'orders', name: 'Orders', permission: 'resources' },
	...TABS.slice(2).map((tab) => ({ ...tab, permission: tab.slug }))
];

export const allowedTabs = (access: Access) => NAV.filter((tab) => access.permissions[tab.permission]);
