import { colour } from '#lib/colors.js';
import { ENEMIES, enemyName, neighbours, type Force, type HexMap } from '#lib/map.js';

export type Hex = [number, number];
export type OrderKind = 'move' | 'spawn' | 'retreat';
export type Status = 'draft' | 'scheduled' | 'resolved' | 'cancelled' | 'rolled_back';
export type Slot = 'warning' | 'followup';

export interface Step {
	row: number;
	col: number;
	arrived: Force;
	lost: Force;
	halted: boolean;
}

export interface Order {
	kind: OrderKind;
	force: Force;
	path: Hex[];
	taken?: Force | null;
	delivered?: Force | null;
	steps?: Step[];
}

export interface Announcement {
	title: string;
	body: string;
	publish_at: string | null;
	on_resolve: boolean;
}

export interface Wave {
	id: number;
	name: string;
	notes: string;
	status: Status;
	resolve_at: string | null;
	resolved_at: string | null;
	created_at: string;
	updated_at: string;
	creator: string | null;
	orders: Order[];
	announcements: Partial<Record<Slot, Announcement>>;
}

export interface Held {
	row: number;
	col: number;
	enemies: Force;
	occupied: boolean;
	last_occupied_at: string | null;
	last_freed_at: string | null;
}

export interface JoelState {
	territory: Held[];
	waves: Wave[];
	latest: number | null;
	now: string;
}

export const STATUSES: Record<Status, string> = {
	draft: 'Draft',
	scheduled: 'Scheduled',
	resolved: 'Resolved',
	cancelled: 'Cancelled',
	rolled_back: 'Rolled back'
};

export const KINDS: Record<OrderKind, string> = { move: 'Move', spawn: 'Spawn', retreat: 'Retreat' };

export const tone = (i: number) => colour(`joel-tone-${(i % 7) + 1}`);

export const key = (row: number, col: number) => `${row},${col}`;

export const total = (force: Force = {}) => ENEMIES.reduce((sum, enemy) => sum + (force[enemy] ?? 0), 0);

export function forceText(force: Force = {}) {
	const parts = ENEMIES.filter((enemy) => force[enemy]).map((enemy) => `${force[enemy]}x ${enemyName(enemy)}`);
	if (!parts.length) return 'nothing';
	return parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}`;
}

export function add(to: Force, more: Force = {}) {
	for (const enemy of ENEMIES) if (more[enemy]) to[enemy] = (to[enemy] ?? 0) + more[enemy];
}

export function take(from: Force, want: Force = {}) {
	const taken: Force = {};
	for (const enemy of ENEMIES) {
		const n = Math.min(want[enemy] ?? 0, from[enemy] ?? 0);
		if (!n) continue;
		taken[enemy] = n;
		from[enemy]! -= n;
		if (!from[enemy]) delete from[enemy];
	}
	return taken;
}

export function simulate(enemies: Map<string, Force>, orders: Order[]) {
	const after = new Map([...enemies].map(([at, force]) => [at, { ...force }]));
	const at = ([row, col]: Hex) => {
		if (!after.has(key(row, col))) after.set(key(row, col), {});
		return after.get(key(row, col))!;
	};
	const results = orders.map((order) => {
		if (!order.path.length) return { taken: {}, short: false };
		const source = at(order.path[0]);
		if (order.kind === 'spawn') {
			add(source, order.force);
			return { taken: { ...order.force }, short: false };
		}
		const taken = take(source, order.force);
		if (order.kind === 'move') add(at(order.path.at(-1)!), taken);
		return { taken, short: total(taken) < total(order.force) };
	});
	return { after, results };
}

export function route(from: Hex, to: Hex, placed: Set<string>) {
	const previous = new Map<string, string | null>([[key(...from), null]]);
	const queue = [from];
	for (const [row, col] of queue) {
		if (row === to[0] && col === to[1]) break;
		for (const [r, c] of neighbours(row, col)) {
			if (!placed.has(key(r, c)) || previous.has(key(r, c))) continue;
			previous.set(key(r, c), key(row, col));
			queue.push([r, c]);
		}
	}
	if (!previous.has(key(...to))) return [];
	const path: Hex[] = [];
	for (let at: string | null = key(...to); at && at !== key(...from); at = previous.get(at) ?? null) {
		path.unshift(at.split(',').map(Number) as Hex);
	}
	return path;
}

export const adjacent = (a: Hex, b: Hex) => neighbours(...a).some(([r, c]) => r === b[0] && c === b[1]);

export function regions(map: HexMap) {
	return new Map((map.regions ?? []).flatMap((region) => region.tiles.map(([row, col]) => [key(row, col), region.name])));
}

export const placeName = (names: Map<string, string>, [row, col]: Hex) => names.get(key(row, col)) ?? `tile ${row}, ${col}`;

export function warning(orders: Order[], names: Map<string, string>) {
	const lines = orders.map((order) => {
		const from = placeName(names, order.path[0]);
		if (order.kind === 'spawn') return `${forceText(order.force)} have been sighted landing in ${from}.`;
		if (order.kind === 'retreat') return `${forceText(order.force)} are pulling back from ${from}.`;
		return `${forceText(order.force)} are advancing from ${from} toward ${placeName(names, order.path.at(-1)!)}.`;
	});
	return { title: 'The Dyconids are on the move', body: lines.join('\n') };
}

export function followup(orders: Order[], names: Map<string, string>) {
	const lines = orders.map((order) => {
		const from = placeName(names, order.path[0]);
		const taken = order.taken ?? order.force;
		if (order.kind === 'spawn') return `${forceText(taken)} landed in ${from}.`;
		if (order.kind === 'retreat') return `${forceText(taken)} withdrew from ${from}.`;
		const delivered = order.delivered ?? taken;
		const last = order.steps?.at(-1);
		const end: Hex = last ? [last.row, last.col] : order.path.at(-1)!;
		const target = order.path.at(-1)!;
		if (!total(delivered)) return `${forceText(taken)} set out from ${from}, but none of them made it.`;
		const lost: Force = {};
		for (const step of order.steps ?? []) add(lost, step.lost);
		const losses = total(lost) ? `, losing ${forceText(lost)} on the way` : '';
		const stopped = end[0] !== target[0] || end[1] !== target[1] ? ` before being stopped short of ${placeName(names, target)}` : '';
		return `${forceText(delivered)} reached ${placeName(names, end)}${losses}${stopped}.`;
	});
	return { title: 'The Dyconid advance is over', body: lines.join('\n') };
}

export function localInput(iso: string | null) {
	if (!iso) return '';
	const date = new Date(iso);
	return new Date(date.getTime() - date.getTimezoneOffset() * 60000).toISOString().slice(0, 16);
}

export const fromInput = (value: string) => (value ? new Date(value).toISOString() : null);

export function relative(iso: string, now = Date.now()) {
	const minutes = Math.round((new Date(iso).getTime() - now) / 60000);
	const size = Math.abs(minutes);
	const text = size < 1 ? 'less than a minute' : size < 60 ? `${size}m` : size < 1440 ? `${Math.floor(size / 60)}h ${size % 60}m` : `${Math.floor(size / 1440)}d ${Math.floor((size % 1440) / 60)}h`;
	return minutes >= 0 ? `in ${text}` : `${text} ago`;
}

export const stamp = (iso: string) => new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });

export function rings(tiles: { row: number; col: number }[], placed: Set<string>, enemies: Map<string, Force>) {
	const occupied = (row: number, col: number) => total(enemies.get(key(row, col))) > 0;
	const found = new Map<string, number>();
	const queue = tiles.filter(({ row, col }) => occupied(row, col) && neighbours(row, col).some(([r, c]) => placed.has(key(r, c)) && !occupied(r, c)));
	for (const { row, col } of queue) found.set(key(row, col), 0);
	for (const { row, col } of queue) {
		for (const [r, c] of neighbours(row, col)) {
			if (!occupied(r, c) || found.has(key(r, c))) continue;
			found.set(key(r, c), found.get(key(row, col))! + 1);
			queue.push({ row: r, col: c });
		}
	}
	return found;
}

export async function request(method: string, url: string, body?: unknown) {
	const response = await fetch(url, {
		method,
		headers: body === undefined ? {} : { 'content-type': 'application/json' },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
	if (!response.ok) throw new Error((await response.text()) || `${response.status}`);
	return response.headers.get('content-type')?.includes('json') ? response.json() : null;
}
