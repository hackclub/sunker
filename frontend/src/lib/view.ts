import { CORNERS, HEIGHT, WIDTH, bounds, neighbours, center, type PlacedTile } from '#lib/map.js';

export const PADDING = 4;
export const OVERSCAN = 1.5;
export const TILT = 20;
export const PERSPECTIVE = 1200;
export const LIFT = Math.tan((TILT * Math.PI) / 180);
export const BACKGROUND = 0.9;
export const FLIGHT = 1800;

const TABLE = 0.24;
const FLATTEN = 0.5;
const RISE = 0.012;

export type Plane = { width: number; height: number; scale: number; x: number; y: number };
type Point = [number, number];

export function frame(tiles: PlacedTile[], width: number, height: number): Plane {
	const box = bounds(tiles);
	const scale = Math.min(width / (box.width + PADDING * 2), height / (box.height + PADDING * 2));
	return {
		width: width * OVERSCAN,
		height: height * OVERSCAN,
		scale,
		x: (width * OVERSCAN) / 2 - (box.left + box.width / 2) * scale,
		y: (height * OVERSCAN) / 2 - (box.top + box.height / 2) * scale
	};
}

export function unprojector(plane: Plane, width: number, height: number) {
	const radians = (TILT * Math.PI) / 180;
	return (sx: number, sy: number): Point => {
		const v = sy - height / 2;
		const ry = v / (Math.cos(radians) + (v * Math.sin(radians)) / PERSPECTIVE);
		const rx = (sx - width / 2) * (1 - (ry * Math.sin(radians)) / PERSPECTIVE);
		return [(rx + plane.width / 2 - plane.x) / plane.scale, (ry + plane.height / 2 - plane.y) / plane.scale];
	};
}

export function grid(plane: Plane, width: number, height: number, reach = 0) {
	const unproject = unprojector(plane, width, height);
	const corners = [unproject(0, 0), unproject(width, 0), unproject(0, height), unproject(width, height)];
	const [xs, ys] = [corners.map(([x]) => x), corners.map(([, y]) => y)];
	return {
		rows: [Math.floor(Math.min(...ys) / (HEIGHT * 0.75)) - 1, Math.ceil((Math.max(...ys) + reach) / (HEIGHT * 0.75)) + 1],
		cols: [Math.floor(Math.min(...xs) / WIDTH) - 1, Math.ceil(Math.max(...xs) / WIDTH) + 1]
	};
}

export function projector(plane: Plane, width: number, height: number) {
	const radians = (TILT * Math.PI) / 180;
	return (x: number, y: number): Point => {
		const rx = x * plane.scale + plane.x - plane.width / 2;
		const ry = y * plane.scale + plane.y - plane.height / 2;
		const w = 1 - (ry * Math.sin(radians)) / PERSPECTIVE;
		return [width / 2 + rx / w, height / 2 + (ry * Math.cos(radians)) / w];
	};
}

function placement(tiles: PlacedTile[], width: number, height: number) {
	return { scale: (TABLE * width) / (bounds(tiles).width * frame(tiles, width, height).scale), rise: RISE * width };
}

export function table(tiles: PlacedTile[], width: number, height: number) {
	const { scale, rise } = placement(tiles, width, height);
	return `translateY(${-rise}px) scale(${scale}, ${scale * FLATTEN})`;
}

export function squash(tiles: PlacedTile[], width: number, height: number): [number, number] {
	const { scale } = placement(tiles, width, height);
	return [scale, scale * FLATTEN];
}

function seat(tiles: PlacedTile[], width: number, height: number) {
	const { scale, rise } = placement(tiles, width, height);
	return ([x, y]: Point): Point => [width / 2 + (x - width / 2) * scale, height / 2 + (y - height / 2) * scale * FLATTEN - rise];
}

export function halo(outline: Point[][], tiles: PlacedTile[], width: number, height: number, keep: (tile: PlacedTile) => boolean = () => true) {
	const point = seat(tiles, width, height);
	return outline
		.filter((_, i) => keep(tiles[i]))
		.map((polygon) => 'M' + polygon.map((p) => point(p).join(' ')).join('L') + 'Z')
		.join('');
}

export function crest(outline: Point[][], tiles: PlacedTile[], width: number, height: number) {
	const point = seat(tiles, width, height);
	return Math.min(...outline.flatMap((polygon) => polygon.map((p) => point(p)[1])));
}

export function island(tiles: PlacedTile[], width: number, height: number) {
	const project = projector(frame(tiles, width, height), width, height);
	const bottom = floor(tiles);
	return tiles.map(({ x, y, tile }) => {
		const lift = tile.height * LIFT;
		return CORNERS.map(([cx, cy], i) => project(x + cx, y + cy - (i === 0 || i === 1 || i === 5 ? lift : Math.min(bottom, lift))));
	});
}

function floor(tiles: PlacedTile[]) {
	return Math.min(...tiles.map((t) => t.tile.height)) * BACKGROUND * LIFT;
}

const path = (points: Point[]) => 'M' + points.map(([x, y]) => `${Math.round(x)} ${Math.round(y)}`).join('L') + 'Z';

export function footprint(outline: Point[][]) {
	return `path('${outline.map(path).join('')}')`;
}

export function inside(polygons: Point[][], [px, py]: Point) {
	return polygons.some((polygon) =>
		polygon.every(([ax, ay], i) => {
			const [bx, by] = polygon[(i + 1) % polygon.length];
			return (bx - ax) * (py - ay) - (by - ay) * (px - ax) >= 0;
		})
	);
}

export function reveal(tiles: PlacedTile[], width: number, height: number) {
	if (!width || !height) return { grow: [], spread: [] };
	const plane = frame(tiles, width, height);
	const project = projector(plane, width, height);
	const { rows, cols } = grid(plane, width, height);
	const lift = floor(tiles) / 2;
	const distance = new Map(tiles.map((t) => [`${t.row},${t.col}`, 0]));
	const queue = tiles.map((t) => [t.row, t.col]);
	for (const [row, col] of queue) {
		const d = distance.get(`${row},${col}`)!;
		for (const [r, c] of neighbours(row, col)) {
			if (r < rows[0] || r > rows[1] || c < cols[0] || c > cols[1] || distance.has(`${r},${c}`)) continue;
			distance.set(`${r},${c}`, d + 1);
			queue.push([r, c]);
		}
	}
	const visible = [...distance].filter(([key]) => {
		const [row, col] = key.split(',').map(Number);
		const [x, y] = project(center(row, col).x, center(row, col).y);
		return x > -width * 0.1 && x < width * 1.1 && y > -height * 0.1 && y < height * 1.1;
	});
	const rings = Math.max(...visible.map(([, d]) => d));
	const spot = (row: number, col: number, i: number) => {
		const { x, y } = center(row, col);
		return [x + CORNERS[i % 6][0], y + CORNERS[i % 6][1]];
	};
	const key = ([x, y]: number[]) => `${Math.round(x * 100)},${Math.round(y * 100)}`;
	const spread: Keyframe[] = [{ '--clip': footprint(island(tiles, width, height)), offset: 0, easing: 'step-end' }];
	for (let k = 1; k <= rings; k++) {
		const edges = new Map<string, number[][]>();
		for (const [cell, d] of distance) {
			if (d > k) continue;
			const [row, col] = cell.split(',').map(Number);
			neighbours(row, col).forEach(([r, c], i) => {
				if ((distance.get(`${r},${c}`) ?? Infinity) <= k) return;
				const [from, to] = [spot(row, col, i), spot(row, col, i + 1)];
				edges.set(key(from), [...(edges.get(key(from)) ?? []), to]);
			});
		}
		let shape = '';
		for (const [start, outgoing] of edges) {
			while (outgoing.length) {
				const loop: Point[] = [];
				let next = outgoing.pop()!;
				while (true) {
					const [x, y] = next;
					loop.push(project(x, y - lift));
					if (key(next) === start) break;
					next = edges.get(key(next))!.pop()!;
				}
				shape += path(loop);
			}
		}
		spread.push({ '--clip': `path('${shape}')`, offset: 0.5 + (0.5 * k) / (rings + 1), easing: 'step-end' });
	}
	spread.push({ '--clip': 'inset(0)', offset: 1 });
	const grow: Keyframe[] = [
		{ transform: table(tiles, width, height), offset: 0, easing: 'cubic-bezier(0.6, 0, 0.3, 1)' },
		{ transform: 'none', offset: 0.5 },
		{ transform: 'none', offset: 1 }
	];
	return { grow, spread };
}
