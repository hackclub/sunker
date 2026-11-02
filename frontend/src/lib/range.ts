import { BASES } from '#lib/bases.js';
import { MAP, rgb } from '#lib/colors.js';
import { RANGE_STRIDE } from '#lib/hexes.js';
import { HEIGHT, WIDTH, center, type PlacedTile } from '#lib/map.js';
import { LIFT } from '#lib/view.js';

const SPACING = 0.2;
const REACH = 1.6;
const SNOW = 1.05;
const LIGHT = [-0.6, 0.35, 0.72].map((v, _, l) => v / Math.hypot(...l));

function hash(x: number, y: number, k: number) {
	let h = Math.imul(x, 374761393) + Math.imul(y, 668265263) + Math.imul(k, 1442695041);
	h = Math.imul(h ^ (h >>> 13), 1274126177);
	return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

const fade = (t: number) => t * t * t * (t * (t * 6 - 15) + 10);
const smoothstep = (a: number, b: number, v: number) => {
	const t = Math.max(0, Math.min(1, (v - a) / (b - a)));
	return t * t * (3 - 2 * t);
};

function noise(x: number, y: number, k: number) {
	const [ix, iy] = [Math.floor(x), Math.floor(y)];
	const [fx, fy] = [x - ix, y - iy];
	const corner = (i: number, j: number) => {
		const a = hash(ix + i, iy + j, k) * Math.PI * 2;
		return Math.cos(a) * (fx - i) + Math.sin(a) * (fy - j);
	};
	const [u, v] = [fade(fx), fade(fy)];
	const top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * u;
	const bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * u;
	return top + (bottom - top) * v;
}

function ridged(x: number, y: number) {
	const [wx, wy] = [x + 0.4 * noise(x * 0.6, y * 0.6, 7), y + 0.4 * noise(x * 0.6, y * 0.6, 8)];
	let total = 0;
	for (const [f, a, k] of [
		[0.5, 0.62, 1],
		[1.1, 0.3, 2],
		[2.4, 0.08, 3]
	])
		total += a * Math.max(0, 1 - Math.abs(noise(wx * f, wy * f, k)) * 1.5) ** 2;
	return Math.min(1, total) ** 1.7;
}

export function raise(tiles: PlacedTile[], placed: Map<string, PlacedTile>) {
	const peaks = tiles.filter(({ tile }) => tile.variants.includes('mountain'));
	if (!peaks.length) return new Float32Array();
	const [rock, snow] = [rgb(MAP.rock), rgb(MAP.snow)];
	const grounds = new Map(peaks.map(({ tile }) => [tile.base, rgb(BASES[tile.base in BASES ? tile.base : 'plain'].flat)]));

	const field = (x: number, y: number) => {
		const row = Math.round(y / (HEIGHT * 0.75));
		let [height, mountain, nearest] = [0, 0, Infinity];
		let home: { x: number; y: number; spot?: PlacedTile } = { x, y };
		const outside: { x: number; y: number }[] = [];
		const ground = [0, 0, 0];
		for (let r = row - 2; r <= row + 2; r++) {
			const col = Math.round(x / WIDTH - (r & 1) / 2);
			for (let c = col - 2; c <= col + 2; c++) {
				const at = center(r, c);
				const distance = Math.hypot(at.x - x, at.y - y);
				const spot = placed.get(`${r},${c}`);
				if (distance < nearest) [nearest, home] = [distance, { ...at, spot }];
				if (!spot?.tile.variants.includes('mountain')) outside.push(at);
				const w = Math.max(0, 1 - distance / REACH) ** 2;
				if (!w || !spot?.tile.variants.includes('mountain')) continue;
				mountain += w;
				height += w * spot.tile.height;
				grounds.get(spot.tile.base)!.forEach((v, i) => (ground[i] += v * w));
			}
		}
		const own = !!home.spot?.tile.variants.includes('mountain');
		const edge = Math.min(...outside.map((at) => (Math.hypot(at.x - x, at.y - y) ** 2 - nearest ** 2) / (2 * Math.hypot(at.x - home.x, at.y - home.y))));
		const mask = own ? smoothstep(0, 0.5, edge) : 0;
		const floor = home.spot?.tile.height ?? 0;
		const base = mountain ? floor + (height / mountain - floor) * mask : floor;
		const amplitude = 0.7 + 0.14 * base;
		return { own, home: home.y, base, mask, amplitude, rise: amplitude * mask * (0.18 + 0.82 * ridged(x, y)), ground: ground.map((v) => (mountain ? v / mountain : 0)) };
	};

	const xs = peaks.map((p) => p.x);
	const ys = peaks.map((p) => p.y);
	const [left, top] = [Math.min(...xs) - REACH, Math.min(...ys) - REACH];
	const columns = Math.ceil((Math.max(...xs) + REACH - left) / SPACING) + 1;
	const rows = Math.ceil((Math.max(...ys) + REACH - top) / (SPACING * 0.866)) + 1;
	const grid = Array.from({ length: rows }, (_, j) =>
		Array.from({ length: columns }, (_, i) => {
			const x = left + (i + (j & 1) / 2 + (hash(i, j, 90) - 0.5) * 0.3) * SPACING;
			const y = top + (j + (hash(i, j, 91) - 0.5) * 0.3) * SPACING * 0.866;
			const f = field(x, y);
			return { x, y, z: f.base * LIFT + f.rise, depth: Math.max(y, f.home), ...f };
		})
	);

	const snowline = grid.flat().reduce((top, p) => Math.max(top, p.z), 0) - SNOW;
	const faces: number[][] = [];
	const face = (a: (typeof grid)[0][0], b: typeof a, c: typeof a) => {
		if (!a.own || !b.own || !c.own) return;
		const [ux, uy, uz] = [b.x - a.x, b.y - a.y, b.z - a.z];
		const [vx, vy, vz] = [c.x - a.x, c.y - a.y, c.z - a.z];
		let n = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx];
		if (n[2] < 0) n = n.map((v) => -v);
		const light = n.reduce((sum, v, i) => sum + v * LIGHT[i], 0) / Math.hypot(...n);
		const band = light > 0.7 ? 1.08 : light > 0.35 ? 0.88 : 0.66;
		const [base, amplitude, rise, mask] = ['base', 'amplitude', 'rise', 'mask'].map((key) => (a[key as 'base'] + b[key as 'base'] + c[key as 'base']) / 3);
		const jitter = noise((a.x + b.x + c.x) * 0.8, (a.y + b.y + c.y) * 0.8, 9);
		const height = rise / amplitude + 0.06 * jitter;
		const altitude = base * LIFT + rise + 0.1 * jitter;
		const ground = [0, 1, 2].map((i) => (a.ground[i] + b.ground[i] + c.ground[i]) / 3);
		let colour: number[];
		if (altitude > snowline) colour = snow.map((v, i) => v * Math.min(1, band) * (band < 0.9 ? [0.82, 0.86, 0.96][i] : 1));
		else if (height < 0.24) colour = ground.map((v) => v * band);
		else if (height < 0.33) colour = ground.map((v, i) => ((v + rock[i]) / 2) * band);
		else colour = rock.map((v) => v * band);
		const alpha = Math.min(1, mask / 0.25);
		faces.push([...[a, b, c].flatMap((p) => [p.x, p.y, p.z, p.depth, ...colour, alpha])]);
	};
	for (let j = 0; j < rows - 1; j++) {
		for (let i = 0; i < columns - 1; i++) {
			const [a, b, c, d] = [grid[j][i], grid[j][i + 1], grid[j + 1][i], grid[j + 1][i + 1]];
			if (j & 1) {
				face(a, d, c);
				face(a, b, d);
			} else {
				face(a, b, c);
				face(b, d, c);
			}
		}
	}
	faces.sort((p, q) => p[3] + p[11] + p[19] - (q[3] + q[11] + q[19]));
	const data = new Float32Array(faces.length * RANGE_STRIDE * 3);
	faces.forEach((f, i) => data.set(f, i * RANGE_STRIDE * 3));
	return data;
}
