export type Days = Record<string, number>;

type Buckets = {
	series: Record<string, Days>;
	breakdowns: Record<'ships_by_ring' | 'resources' | 'spent' | 'countries', Record<string, Days>>;
};

export type Analytics = Buckets & {
	today: string;
	hour: string;
	hourly: Buckets;
	activity: { chat: Days; coding: Days };
	guilds: { name: string; value: number }[];
	held: Record<string, number>;
	hours_building: number;
};

export type Unit = 'count' | 'hours' | 'percent' | 'nps';
export type Granularity = 'hour' | 'day' | 'week' | 'total';
export type Preset = '24h' | '7d' | '30d' | '90d' | 'all' | 'custom';
export type Range = { start: string; end: string };
export type Point = { label: string; short: string; value: number | null };

export type Metric = {
	key: string;
	title: string;
	description: string;
	kind: 'flow' | 'level' | 'ratio';
	add: string[];
	sub?: string[];
	base?: string;
	unit?: Unit;
	upIsBad?: boolean;
	note?: string;
	daily?: boolean;
	parts?: { name: string; key: string }[];
};

export type Row = { name: string; value: number; detail?: string };

const DAY_MS = 86_400_000;
const HOUR_MS = 3_600_000;
const HACKATIME = 'No data yet: needs the Hackatime sync';

const isHour = (key: string) => key.length > 10;
export const hourly = (range: Range) => isHour(range.start);
const parse = (key: string) => Date.parse(isHour(key) ? `${key}:00Z` : key);
const toKey = (time: number, hour: boolean) => new Date(time).toISOString().slice(0, hour ? 13 : 10);
const step = (key: string) => (isHour(key) ? HOUR_MS : DAY_MS);
const shift = (key: string, n: number) => toKey(parse(key) + n * step(key), isHour(key));
const span = (range: Range) => Math.round((parse(range.end) - parse(range.start)) / step(range.start)) + 1;

function* keys(start: string, end: string) {
	for (let time = parse(start); time <= parse(end); time += step(start)) yield toKey(time, isHour(start));
}

export const source = (data: Analytics, range: Range): Buckets => (hourly(range) ? data.hourly : data);

const weekOf = (day: string, start: string) => shift(start, Math.floor((span({ start, end: day }) - 1) / 7) * 7);

const shortDate = (day: string) =>
	new Date(day).toLocaleDateString('en', { month: 'short', day: 'numeric', timeZone: 'UTC' });
const longDate = (day: string) =>
	new Date(day).toLocaleDateString('en', { month: 'short', day: 'numeric', year: 'numeric', timeZone: 'UTC' });
const shortLabel = (key: string) => (isHour(key) ? `${key.slice(11)}:00` : shortDate(key));
const longLabel = (key: string) => (isHour(key) ? `${longDate(key.slice(0, 10))}, ${key.slice(11)}:00 UTC` : longDate(key));

export function rangeLabel(range: Range, today: string) {
	if (hourly(range)) return `Last ${span(range)} hours`;
	if (range.end === today) {
		const length = span(range);
		if (length === 1) return 'Today';
		return `Last ${length} days`;
	}
	return `${longDate(range.start)} to ${longDate(range.end)}`;
}

function firstDay(data: Analytics) {
	const all = [
		...Object.values(data.series),
		...Object.values(data.breakdowns).flatMap((categories) => Object.values(categories))
	].flatMap((series) => Object.keys(series));
	return all.reduce((first, day) => (day < first ? day : first), data.today);
}

export function presetRange(preset: Exclude<Preset, 'custom'>, data: Analytics): Range {
	if (preset === '24h') return { start: shift(data.hour, -23), end: data.hour };
	const end = data.today;
	if (preset === 'all') {
		const start = firstDay(data);
		const week = shift(end, -6);
		return { start: start < week ? start : week, end };
	}
	return { start: shift(end, 1 - parseInt(preset)), end };
}

export function chosenRange(value: string | null, fallback: Exclude<Preset, 'custom'>, data: Analytics): { preset: Preset; range: Range } {
	const custom = value?.match(/^(\d{4}-\d{2}-\d{2})\.\.(\d{4}-\d{2}-\d{2})$/);
	if (custom && custom[1] <= custom[2] && !isNaN(Date.parse(custom[1])) && !isNaN(Date.parse(custom[2])))
		return { preset: 'custom', range: { start: custom[1], end: custom[2] } };
	const preset = (['24h', '7d', '30d', '90d', 'all'] as const).find((preset) => preset === value) ?? fallback;
	return { preset, range: presetRange(preset, data) };
}

export function versus(range: Range) {
	const length = span(range);
	const unit = hourly(range) ? 'hour' : 'day';
	return length === 1 ? `vs the ${unit} before` : `vs the ${length} ${unit}s before`;
}

export const previousRange = (range: Range): Range => {
	const length = span(range);
	return { start: shift(range.start, -length), end: shift(range.start, -1) };
};

const total = (from: Record<string, Days>, names: string[] = []) =>
	names.reduce((sum, name) => sum + Object.values(from[name] ?? {}).reduce((a, b) => a + b, 0), 0);

export function points(
	metric: Metric,
	data: Analytics,
	range: Range,
	granularity: Granularity,
	cumulative = false
): Point[] {
	const hour = hourly(range);
	if (hour && metric.daily)
		return granularity === 'total'
			? []
			: [...keys(range.start, range.end)].map((key) => ({ label: longLabel(key), short: shortLabel(key), value: null }));
	const { series } = source(data, range);
	const get = (key: string, at: string) => series[key]?.[at] ?? 0;
	const first = hour ? shift(data.hour, -47) : firstDay(data);
	const before = (names: string[] = []) => (hour ? total(data.series, names) - total(data.hourly.series, names) : 0);
	let runNum = before(metric.add) - before(metric.sub);
	let runDen = metric.base ? before([metric.base]) : 0;
	const buckets = new Map<string, { start: string; num: number; den: number; runNum: number; runDen: number }>();
	for (const at of keys(first < range.start ? first : range.start, range.end)) {
		const num =
			metric.add.reduce((sum, key) => sum + get(key, at), 0) -
			(metric.sub ?? []).reduce((sum, key) => sum + get(key, at), 0);
		const den = metric.base ? get(metric.base, at) : 0;
		runNum += num;
		runDen += den;
		if (at < range.start) continue;
		const key = granularity === 'week' ? weekOf(at, range.start) : granularity === 'total' ? 'total' : at;
		const bucket = buckets.get(key) ?? { start: at, num: 0, den: 0, runNum: 0, runDen: 0 };
		bucket.num += num;
		bucket.den += den;
		bucket.runNum = runNum;
		bucket.runDen = runDen;
		buckets.set(key, bucket);
	}
	return [...buckets.values()].map((bucket) => {
		let value: number | null;
		if (metric.kind === 'level') value = bucket.runNum;
		else if (metric.kind === 'ratio') {
			const [num, den] = cumulative ? [bucket.runNum, bucket.runDen] : [bucket.num, bucket.den];
			value = den ? num / den : null;
		} else value = cumulative ? bucket.runNum : bucket.num;
		const label = granularity === 'week' ? `Week of ${longDate(bucket.start)}` : longLabel(bucket.start);
		return { label, short: shortLabel(bucket.start), value };
	});
}

export const headline = (metric: Metric, data: Analytics, range: Range, cumulative = false) =>
	points(metric, data, range, 'total', cumulative)[0]?.value ?? null;

export function delta(metric: Metric, current: number | null, previous: number | null) {
	if (current === null || previous === null) return null;
	if (current === previous) return { text: 'No change', tone: 'flat' };
	const up = current > previous;
	const tone = up !== !!metric.upIsBad ? 'good' : 'bad';
	if (metric.unit === 'nps') return { text: `${up ? '+' : '-'}${Math.round(Math.abs(current - previous) * 100)} pts`, tone };
	if (previous === 0) return { text: 'New', tone };
	const change = (current - previous) / Math.abs(previous);
	return { text: `${up ? '+' : '-'}${formatNumber(Math.abs(change) * 100)}%`, tone };
}

export function totals(categories: Record<string, Days>, names: string[], range: Range): Row[] {
	return names.map((name) => {
		let value = 0;
		for (const at of keys(range.start, range.end)) value += categories[name]?.[at] ?? 0;
		return { name, value };
	});
}

const formatNumber = (value: number) =>
	Math.abs(value) < 10_000
		? value.toLocaleString('en', { maximumFractionDigits: Math.abs(value) < 10 ? 1 : 0 })
		: value.toLocaleString('en', { notation: 'compact', maximumFractionDigits: 1 });

export function format(value: number | null, unit: Unit = 'count') {
	if (value === null) return 'n/a';
	if (unit === 'percent') return `${Math.round(value * 100)}%`;
	if (unit === 'hours') return `${formatNumber(value)}h`;
	if (unit === 'nps') return `${value > 0 ? '+' : ''}${Math.round(value * 100)}`;
	return formatNumber(value);
}

export const METRICS: Record<string, Metric> = Object.fromEntries(
	(
		[
			{
				key: 'signups',
				title: 'Signups',
				description: 'New accounts created',
				kind: 'flow',
				add: ['signups']
			},
			{
				key: 'logins',
				title: 'Logins',
				description: 'Unique people who opened HQ, once per UTC day',
				kind: 'flow',
				add: ['logins']
			},
			{
				key: 'repeat_logins',
				title: 'Repeat logins',
				description: 'Logins minus first-time logins',
				kind: 'flow',
				add: ['repeat_logins'],
				daily: true
			},
			{
				key: 'dau',
				title: 'DAU',
				description: 'People who logged 5+ minutes on linked Hackatime projects in a UTC day',
				kind: 'flow',
				add: ['dau'],
				note: HACKATIME,
				daily: true
			},
			{
				key: 'referrals',
				title: 'Referral signups',
				description: 'Signups that came through a referral code',
				kind: 'flow',
				add: ['referrals']
			},
			{
				key: 'retention_1',
				title: 'Day 1 retention',
				description: 'Share of signups who logged in again 1 or more days later',
				kind: 'ratio',
				add: ['retention_1'],
				base: 'retention_1_base',
				unit: 'percent',
				daily: true
			},
			{
				key: 'retention_7',
				title: 'Day 7 retention',
				description: 'Share of signups who logged in again 7 or more days later',
				kind: 'ratio',
				add: ['retention_7'],
				base: 'retention_7_base',
				unit: 'percent',
				daily: true
			},
			{
				key: 'duc',
				title: 'DUC',
				description: 'Unique people who sent a message in any chat',
				kind: 'flow',
				add: ['duc']
			},
			{
				key: 'messages',
				title: 'Chat messages',
				description: 'Messages sent across all chats',
				kind: 'flow',
				add: ['messages']
			},
			{
				key: 'feedback',
				title: 'Feedback NPS',
				description: 'Net promoter score: % promoters (9 to 10) minus % detractors (6 or below), from -100 to +100',
				kind: 'ratio',
				add: ['feedback_promoters'],
				sub: ['feedback_detractors'],
				base: 'feedback_count',
				unit: 'nps',
				parts: [
					{ name: 'Promoters (9 to 10)', key: 'feedback_promoters' },
					{ name: 'Neutrals (7 to 8)', key: 'feedback_passives' },
					{ name: 'Detractors (1 to 6)', key: 'feedback_detractors' }
				]
			},
			{
				key: 'projects',
				title: 'Projects created',
				description: 'New projects started',
				kind: 'flow',
				add: ['projects']
			},
			{
				key: 'shipped',
				title: 'Shipped projects',
				description: 'Ships submitted for review',
				kind: 'flow',
				add: ['shipped']
			},
			{
				key: 'approved',
				title: 'Approved projects',
				description: 'Ships approved by a reviewer',
				kind: 'flow',
				add: ['approved']
			},
			{
				key: 'rejected',
				title: 'Rejected projects',
				description: 'Ships rejected by a reviewer',
				kind: 'flow',
				add: ['rejected'],
				upIsBad: true
			},
			{
				key: 'pending',
				title: 'Pending projects',
				description: 'Ships waiting for a review decision',
				kind: 'level',
				add: ['shipped'],
				sub: ['approved', 'rejected'],
				upIsBad: true
			},
			{
				key: 'hours_approved',
				title: 'Hours approved',
				description: 'Approved time on decided ships',
				kind: 'flow',
				add: ['hours_approved'],
				unit: 'hours'
			},
			{
				key: 'outstanding',
				title: 'Outstanding prizes',
				description: 'Prize orders not fulfilled yet',
				kind: 'level',
				add: ['orders'],
				sub: ['orders_fulfilled'],
				upIsBad: true
			},
			{
				key: 'fulfilled',
				title: 'Orders fulfilled',
				description: 'Prize orders marked fulfilled',
				kind: 'flow',
				add: ['orders_fulfilled']
			}
		] satisfies Metric[]
	).map((metric) => [metric.key, metric])
);

const FUNNEL: { name: string; key: string; note?: string }[] = [
	{ name: 'Visited', key: 'funnel_visit' },
	{ name: 'Logged in', key: 'signups' },
	{ name: 'Completed onboarding', key: 'funnel_onboard' },
	{ name: 'Linked Hackatime', key: 'funnel_hackatime', note: HACKATIME },
	{ name: 'Created a project', key: 'funnel_project' },
	{ name: 'Logged time', key: 'funnel_time', note: HACKATIME },
	{ name: 'Shipped a project', key: 'funnel_ship' },
	{ name: 'Redeemed a prize', key: 'funnel_prize' }
];

export function funnel(data: Analytics, range: Range): Row[] {
	const { series } = source(data, range);
	const rows = totals(
		Object.fromEntries(FUNNEL.map((stage) => [stage.name, series[stage.key] ?? {}])),
		FUNNEL.map((stage) => stage.name),
		range
	);
	return rows.map((row, i) => {
		const previous = rows[i - 1]?.value;
		const detail =
			previous === undefined ? undefined : previous ? `${format(row.value / previous, 'percent')} of previous` : undefined;
		return { ...row, detail };
	});
}

const RESOURCE_NAMES: Record<string, string> = {
	chiprock: 'Chiprock',
	granite: 'Granite',
	potash: 'Potash',
	bauxite: 'Bauxite',
	shale: 'Shale',
	kalkite: 'Kalkite',
	cuprite: 'Cuprite',
	ring_upgrade: 'Ring upgrades',
	scrip: 'Scrip'
};

export const BREAKDOWNS = {
	ships_by_ring: {
		title: 'Ships per ring',
		description: 'Ships submitted at each ring, 0 is hardest',
		names: [0, 1, 2, 3, 4, 5].map((ring) => `Ring ${ring}`),
		label: (name: string) => name
	},
	resources: {
		title: 'Resources granted',
		description: 'Resources given out through the ledger',
		names: Object.keys(RESOURCE_NAMES),
		label: (name: string) => RESOURCE_NAMES[name] ?? name
	},
	spent: {
		title: 'Resources spent',
		description: 'Resources taken out through the ledger',
		names: Object.keys(RESOURCE_NAMES),
		label: (name: string) => RESOURCE_NAMES[name] ?? name
	}
};

export const held = (data: Analytics): Row[] =>
	Object.keys(RESOURCE_NAMES).map((name) => ({ name, value: data.held[name] ?? 0 }));

export const shares = (rows: Row[]): Row[] => {
	const all = rows.reduce((sum, row) => sum + row.value, 0);
	return rows.map((row) => ({ ...row, detail: all ? `${Math.round((row.value / all) * 100)}%` : undefined }));
};

export const ACTIVITY = {
	coding: {
		title: 'Coding time',
		description: 'Hours logged on linked Hackatime projects, by weekday and hour (UTC)',
		unit: 'hours',
		noun: 'coded',
		note: HACKATIME
	},
	chat: {
		title: 'Chat activity',
		description: 'Messages sent, by weekday and hour (UTC)',
		unit: 'count',
		noun: 'messages',
		note: undefined
	}
} satisfies Record<string, { title: string; description: string; unit: Unit; noun: string; note?: string }>;

export const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

export function activity(counts: Days, range: Range) {
	const grid = WEEKDAYS.map(() => Array<number>(24).fill(0));
	for (const [key, value] of Object.entries(counts)) {
		const at = hourly(range) ? key : key.slice(0, 10);
		if (at < range.start || at > range.end) continue;
		const date = new Date(parse(key));
		grid[(date.getUTCDay() + 6) % 7][date.getUTCHours()] += value;
	}
	return grid;
}

export const HOURS_NOTE = HACKATIME;

const empty = (data: Analytics, keys: string[]) =>
	keys.every((key) => !Object.keys(data.series[key] ?? {}).length);

export const note = (metric: Metric, data: Analytics) =>
	metric.note && empty(data, metric.add) ? metric.note : undefined;

export function funnelNote(data: Analytics) {
	const missing = FUNNEL.filter((stage) => stage.note && empty(data, [stage.key]));
	if (!missing.length) return undefined;
	return `No data yet for ${missing.map((stage) => stage.name).join(' and ')}: needs the Hackatime sync`;
}

const regions = new Intl.DisplayNames(['en'], { type: 'region' });

export function countryName(code: string) {
	if (code === 'unknown') return 'Unknown';
	try {
		return regions.of(code) ?? code;
	} catch {
		return code;
	}
}

export function countries(data: Analytics, range: Range): Row[] {
	const counts = source(data, range).breakdowns.countries;
	const rows = totals(counts, Object.keys(counts), range).filter((row) => row.value);
	const total = rows.reduce((sum, row) => sum + row.value, 0);
	return rows
		.sort((a, b) => b.value - a.value)
		.map((row) => {
			const share = (row.value / total) * 100;
			return { ...row, detail: `${share.toFixed(share < 10 ? 1 : 0)}%` };
		});
}
