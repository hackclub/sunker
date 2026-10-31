export type Row = Record<string, unknown>;

export type Summary = {
	id: number;
	name: string | null;
	email: string | null;
	slack_id: string | null;
	handle: string | null;
	guild: string | null;
	scrip: number | null;
	projects: number;
	ships: number;
	last_seen: string | null;
	is_banned: boolean;
	deleted_at: string | null;
	created_at: string;
};

export type Person = { id: number; name: string | null };

export type Detail = {
	user: Row & { id: number; first_name: string | null; last_name: string | null; is_banned: boolean; deleted_at: string | null };
	guild: Row | null;
	referrer: Person | null;
	referrals: Person[];
	character: (Row & { handle: string }) | null;
	resources: Record<string, number> | null;
	ledger: Row[];
	ledger_by: Row[];
	projects: Row[];
	ships: Row[];
	reviewed_ships: Row[];
	orders: Row[];
	chats: { id: number; name: string; allowed: boolean }[];
	guilds: { id: number; name: string }[];
	message_count: number;
	messages: Row[];
	analytics_counts: Record<string, number>;
	analytics: Row[];
	admin_actions: Row[];
	admin_actions_by: Row[];
	slack_preference: Record<string, boolean> | null;
	slack_dms: Row[];
	feedback: Row[];
	sessions: Row[];
	auth_tokens: Row[];
};

export type Private = { user: Row; orders: Record<string, Row> };

export const RESOURCES = ['chiprock', 'granite', 'potash', 'bauxite', 'shale', 'kalkite', 'cuprite', 'ring_upgrade', 'scrip'];

export const PRIVATE_FIELDS = [
	'birthday',
	'phone_number',
	'addr_first_name',
	'addr_last_name',
	'addr_line_1',
	'addr_line_2',
	'addr_city',
	'addr_state',
	'addr_postcode',
	'addr_country',
	'addr_phone_number'
];

const TIMESTAMP = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/;

export const isPerson = (value: unknown): value is Person =>
	typeof value === 'object' && value !== null && 'id' in value && 'name' in value && Object.keys(value).length === 2;

export const personName = (person: Person) => person.name ?? `User #${person.id}`;

export function format(value: unknown): string {
	if (value === null || value === undefined) return '';
	if (typeof value === 'boolean') return value ? 'yes' : 'no';
	if (typeof value === 'string' && TIMESTAMP.test(value)) return new Date(value).toLocaleString();
	if (Array.isArray(value)) return value.map(format).join(', ');
	if (typeof value === 'object') return JSON.stringify(value);
	return String(value);
}

export const label = (key: string) => key.replaceAll('_', ' ');

export const duration = (seconds: number) => `${(seconds / 3600).toFixed(1)}h`;
