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
	hostile: '#d42c2c',
	hostileBorder: PALETTE.negative,
	hostileHover: '#ff8f8f',
	hostileLine: '#ffd6cc',
	fog: '#f2b5ad',
	fogShade: '#d8857d',
	fogLight: '#fbe0db',
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

function luminance(color: number[]) {
	const [r, g, b] = color.map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: number[], b: number[]) {
	const [high, low] = [luminance(a), luminance(b)].sort((p, q) => q - p);
	return (high + 0.05) / (low + 0.05);
}

export function legible(ground: string, ratio = 4.5) {
	const base = rgb(ground);
	const dark = contrast(base, [0, 0, 0]) >= contrast(base, [1, 1, 1]);
	let ink = dark ? [0, 0, 0] : [1, 1, 1];
	for (let t = 0; t <= 1; t += 0.02) {
		const tint = base.map((c) => (dark ? c * (1 - t) : c + (1 - c) * t));
		if (contrast(tint, base) >= ratio) {
			ink = tint;
			break;
		}
	}
	return `rgb(${ink.map((c) => Math.round(c * 255)).join(' ')})`;
}

export const vec3 =(color: string) => `vec3(${rgb(color).map((c) => c.toFixed(6))})`;

export const css = (color: string, alpha = 1) => `rgb(${rgb(color).map((c) => c * 255).join(' ')} / ${alpha})`;
