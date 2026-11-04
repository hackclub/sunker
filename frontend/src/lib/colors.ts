import { styles } from '#lib/styles.svelte.js';

export const COLOURS: Record<string, Record<string, string>> = {
	Interface: {
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
		backdrop: '#00000099',
		'table-edge': '#2e1a10',
		sky: '#a8d8f0',
		horizon: '#5a8fc8',
		ring: '#2e3032',
		'ring-label': '#b4b7b9',
		avatar: '#9da1a8'
	},
	Map: {
		'map-hostile': '#d42c2c',
		'map-hostile-hover': '#ff8f8f',
		'map-hostile-line': '#ffd6cc',
		'map-origin': '#e8b730',
		'map-mark': '#ffffff',
		'map-fog': '#f2b5ad',
		'map-fog-shade': '#d8857d',
		'map-fog-light': '#fbe0db',
		'map-line': '#fff8e4',
		'map-asphalt': '#484656',
		'map-paint': '#ffde70',
		'map-rock': '#96909c',
		'map-snow': '#f2f4f8',
		'map-shadow': '#000000',
		'map-side': '#6b4a34',
		'map-hedge': '#34803c',
		'map-field': '#458635',
		'map-crop-1': '#60ba4a',
		'map-crop-2': '#9ed048',
		'map-crop-3': '#f0ca5c',
		'map-crop-4': '#48a454',
		'map-trail': '#ffffff',
		'map-trail-edge': '#000000'
	},
	Terrain: {
		plain: '#3a4466',
		grass: '#6cc25e',
		'grass-low': '#5cb858',
		'grass-high': '#82d068',
		'grass-blade': '#3e9c48',
		brush: '#d2bd6c',
		'brush-low': '#c6b260',
		'brush-high': '#e2cc7a',
		'brush-blade': '#b09648',
		sand: '#f6e2b0',
		'sand-ripple': '#e8ce9a',
		concrete: '#8e8ca0',
		'concrete-speck-light': '#aaaabe',
		'concrete-speck-dark': '#767288',
		dirt: '#8c6a52',
		'dirt-low': '#82624a',
		'dirt-high': '#967256',
		'dirt-speck': '#624636',
		ocean: '#3d8cc4',
		'ocean-deep': '#3a86c0',
		'ocean-light': '#4e9ed2'
	},
	Joel: {
		'joel-tone-1': '#e0904a',
		'joel-tone-2': '#548cde',
		'joel-tone-3': '#c86cd8',
		'joel-tone-4': '#6cc47a',
		'joel-tone-5': '#e0c84a',
		'joel-tone-6': '#4ac8c8',
		'joel-tone-7': '#e06c9a',
		'joel-forest': '#2c6a2c',
		'joel-farmland': '#c8d24a',
		'joel-houses': '#b8aca4',
		'joel-buildings': '#8e8ca0',
		'joel-mountain': '#6e6a78',
		'joel-mine': '#4a3828',
		'joel-highway': '#484656',
		'joel-cactus': '#4a8a3a',
		'joel-label': '#ffffff',
		'joel-hover': '#ffffff'
	},
	Mines: {
		'mines-depth': '#2a2b2d'
	},
	Passport: {
		'passport-cover': '#24344f',
		'passport-paper': '#e9ebee',
		'passport-paper-shade': '#dadde2',
		'passport-guilloche': '#b4b7b9',
		'passport-ink': '#1e1f22',
		'passport-label': '#5c6066',
		'passport-red': '#3574f0',
		'passport-photo': '#c8cbd0',
		'passport-silhouette': '#9da1a8',
		'passport-missing': '#548cde'
	},
	Features: {
		'feature-1': '#2b2b2b',
		'feature-2': '#ffffff',
		'feature-3': '#6b4226',
		'feature-4': '#d9a066',
		'feature-5': '#d94a4a',
		'feature-6': '#f08a3c',
		'feature-7': '#f5d547',
		'feature-8': '#5cb85c',
		'feature-9': '#548cde',
		'feature-10': '#9b59b6'
	}
};

export const DEFAULTS: Record<string, string> = Object.assign({}, ...Object.values(COLOURS));

export const colour = (key: string) => styles.colours[key] ?? DEFAULTS[key];

function view<T extends Record<string, string | string[]>>(keys: T) {
	const object = {} as { readonly [K in keyof T]: T[K] extends string[] ? string[] : string };
	for (const [name, key] of Object.entries(keys))
		Object.defineProperty(object, name, { enumerable: true, get: () => (Array.isArray(key) ? key.map(colour) : colour(key)) });
	return object;
}

export const PALETTE = view(Object.fromEntries(Object.keys(COLOURS.Interface).map((key) => [key, key])));

export const MAP = view({
	background: 'background',
	highlight: 'accent',
	hostile: 'map-hostile',
	hostileBorder: 'negative',
	hostileHover: 'map-hostile-hover',
	hostileLine: 'map-hostile-line',
	origin: 'map-origin',
	mark: 'map-mark',
	fog: 'map-fog',
	fogShade: 'map-fog-shade',
	fogLight: 'map-fog-light',
	line: 'map-line',
	asphalt: 'map-asphalt',
	paint: 'map-paint',
	rock: 'map-rock',
	snow: 'map-snow',
	shadow: 'map-shadow',
	side: 'map-side',
	hedge: 'map-hedge',
	field: 'map-field',
	crops: ['map-crop-1', 'map-crop-2', 'map-crop-3', 'map-crop-4']
});

export function rgb(color: string) {
	const match = /^#([0-9a-f]{3}|[0-9a-f]{6})(?:[0-9a-f]{2})?$/i.exec(color);
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
