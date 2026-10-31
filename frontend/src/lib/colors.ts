export const PALETTE = {
	background: '#0e0f10',
	island: '#1e1f22',
	'island-border': '#2b2d30',
	hover: '#2b2d30',
	'tip-border': '#393b40',
	text: '#dfe1e5',
	'text-muted': '#868a91',
	accent: '#548cde',
	dot: '#3574f0',
	negative: '#e06c6c',
	positive: '#6cc47a',
	flag: '#e0904a',
	shadow: '#00000080',
	outline: '#ffffff',
	'table-edge': '#2e1a10',
	sky: '#a8d8f0',
	horizon: '#5a8fc8',
	ring: '#2e3032',
	'ring-label': '#b4b7b9'
};

export const MAP = {
	background: PALETTE.background,
	highlight: PALETTE.accent,
	line: '#fff8e4',
	asphalt: '#484656',
	paint: '#ffde70',
	rock: '#96909c',
	shadow: '#000000',
	side: '#6b4a34',
	hedge: '#34803c',
	field: '#458635',
	crops: ['#60ba4a', '#9ed048', '#f0ca5c', '#48a454']
};

export function rgb(color: string) {
	const match = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(color);
	if (!match) throw new Error(`Expected a #rgb or #rrggbb colour, got "${color}"`);
	const hex = match[1].length === 3 ? [...match[1]].map((c) => c + c).join('') : match[1];
	return [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
}

export const vec3 = (color: string) => `vec3(${rgb(color).map((c) => c.toFixed(6))})`;

export const css = (color: string, alpha = 1) => `rgb(${rgb(color).map((c) => c * 255).join(' ')} / ${alpha})`;
