import { CORNERS, HEIGHT, WIDTH, bounds, neighbours, center, type PlacedTile } from '#lib/map.js';

export const PADDING = 4;
const EDGE = 1;
const PAN = 0.6;
const ZOOMS = 16;
const STEPS = 30;
export const OVERSCAN = 1.5;
export const TILT = 20;
export const PERSPECTIVE = 1200;
export const LIFT = Math.tan((TILT * Math.PI) / 180);
export const BACKGROUND = 0.9;
export const FLIGHT = 1800;

const TABLE = 0.24;
const FLATTEN = 0.5;
const RISE = 0.012;
export const BAR = 64;
export const PHONE = 600;
const ROW = 76;
const SIDE = 0.64;
const SHARE = 400;
const GUIDE = 0.62;
const ART = { left: 117, top: 28, right: 156 };

export type Plane = { width: number; height: number; scale: number; x: number; y: number };
export type Pose = { x: number; y: number; sx: number; sy: number };
export type Rect = { x: number; y: number; width: number; height: number };
export type Stage = {
	pose: Pose;
	wall: number;
	box: Rect | null;
	unit: number;
	crowd: { left: number; right: number; top: number; bottom: number; person: number; depth: number; pad: number } | null;
	guide: { size: number; top: number } | null;
};
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

export function middle(plane: Plane): Point {
	return [(plane.width / 2 - plane.x) / plane.scale, (plane.height / 2 - plane.y) / plane.scale];
}

export function aim(plane: Plane, zoom: number, [x, y]: Point): Plane {
	const scale = plane.scale * zoom;
	return { ...plane, scale, x: plane.width / 2 - x * scale, y: plane.height / 2 - y * scale };
}

function limits(plane: Plane, tiles: PlacedTile[], width: number, height: number, zoom: number, rest = 0) {
	const unproject = unprojector(plane, width, height);
	const [cx, cy] = middle(plane);
	const corners = [unproject(0, 0), unproject(width, 0), unproject(0, height), unproject(width, height)];
	const span = Math.min(...corners.map(([px]) => Math.abs(px - cx))) / zoom;
	const [up, down] = [(cy - Math.min(...corners.map(([, py]) => py))) / zoom, (Math.max(...corners.map(([, py]) => py)) - cy) / zoom];
	const box = bounds(tiles);
	const slack = zoom - 1;
	const [sideways, upward, downward] = [span, up, down].map((reach) => reach * PAN);
	const [left, right] = [Math.max(box.left - EDGE + span - sideways, cx - slack * span - sideways), Math.min(box.left + box.width + EDGE - span + sideways, cx + slack * span + sideways)];
	const [top, bottom] = [Math.max(box.top - EDGE + up - upward, cy - slack * up - upward), Math.min(box.top + box.height + EDGE - down + downward, cy + slack * down + downward)];
	const drop = rest ? (cy - unproject(width / 2, rest)[1]) / zoom : 0;
	return { xs: left > right ? [cx] : [left, right], ys: top > bottom ? [cy + Math.min(0, drop), cy + Math.max(0, drop)] : [top + Math.min(0, drop), bottom + Math.max(0, drop)], span, up, down, drop };
}

export function confine(plane: Plane, tiles: PlacedTile[], width: number, height: number, zoom: number, point: Point, rest = 0): Point {
	const { xs, ys, span, up, down, drop } = limits(plane, tiles, width, height, zoom, rest);
	const fence = ([x, y]: Point): Point => [Math.max(xs[0], Math.min(xs.at(-1)!, x)), Math.max(ys[0], Math.min(ys.at(-1)!, y))];
	const [x, y] = fence(point);
	const seen = y - drop;
	const near = tiles.reduce((best, t) => (Math.hypot(t.x - x, t.y - seen) < Math.hypot(best.x - x, best.y - seen) ? t : best));
	const reach = Math.min(span, up, down) / 2;
	const distance = Math.hypot(x - near.x, seen - near.y);
	return distance <= reach ? [x, y] : fence([near.x + ((x - near.x) * reach) / distance, near.y + ((seen - near.y) * reach) / distance + drop]);
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

export function grid(plane: Plane, width: number, height: number, reach = 0, tiles: PlacedTile[] = [], deepest = 1) {
	const views = [plane];
	for (let i = 0; tiles.length && i <= ZOOMS; i++) {
		const zoom = 1 + ((deepest - 1) * i) / ZOOMS;
		const { xs, ys } = limits(plane, tiles, width, height, zoom);
		for (const x of xs) for (const y of ys) views.push(aim(plane, zoom, [x, y]));
	}
	const corners = views.flatMap((view) => {
		const unproject = unprojector(view, width, height);
		return [unproject(0, 0), unproject(width, 0), unproject(0, height), unproject(width, height)];
	});
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

function flat(tiles: PlacedTile[], width: number, height: number): Pose {
	const scale = (TABLE * width) / (bounds(tiles).width * frame(tiles, width, height).scale);
	return { x: 0, y: -RISE * width, sx: scale, sy: scale * FLATTEN };
}

function level(tiles: PlacedTile[], width: number, height: number, area: Rect): Pose {
	const points = island(tiles, width, height).flat();
	const [xs, ys] = [points.map(([x]) => x), points.map(([, y]) => y)];
	const [left, right, top, bottom] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
	const scale = Math.min(area.width / (right - left), area.height / ((bottom - top) * FLATTEN));
	const [dx, dy] = [(left + right) / 2 - width / 2, (top + bottom) / 2 - height / 2];
	return { x: area.x + area.width / 2 - width / 2 - scale * dx, y: area.y + area.height / 2 - height / 2 - scale * FLATTEN * dy, sx: scale, sy: scale * FLATTEN };
}

export function stage(tiles: PlacedTile[], width: number, height: number, top: number, bottom: number, hud: number, portrait: boolean): Stage {
	if (!portrait) return { pose: flat(tiles, width, height), wall: height / 3, box: null, unit: 0, crowd: null, guide: null };
	const length = Math.round(Math.min(width * SIDE, 380));
	const unit = length / SHARE;
	const person = Math.max(44, Math.min(length * 0.2, 76));
	const depth = person * 0.45;
	const clear = top + 12 + hud;
	const floor = height - bottom - (width <= PHONE ? BAR : 0) - ROW - 8;
	const long = Math.max(Math.min(floor - (clear + person * 1.74 + depth + 4), length * 1.8), length * 0.6);
	const left = Math.round((width - length) / 2);
	const table = { x: left, y: floor - long, width: length, height: long };
	const wall = Math.round(table.y - 12);
	const size = Math.round(Math.max(72, Math.min(width * 0.22, 120)));
	const area = { x: left + 55 * unit, y: table.y + 30 * unit, width: length - 125 * unit, height: long - 140 * unit };
	return {
		pose: level(tiles, width, height, area),
		wall,
		box: { x: left - ART.left * unit, y: table.y - ART.top * unit, width: length + (ART.left + ART.right) * unit, height: long + ART.top * unit },
		unit,
		crowd: { left, right: left + length, top: table.y, bottom: table.y + long, person, depth, pad: width <= PHONE ? 2 : BAR + 4 },
		guide: { size, top: wall - Math.round(size * GUIDE) }
	};
}

export const transform = (pose: Pose) => `translate(${pose.x}px, ${pose.y}px) scale(${pose.sx}, ${pose.sy})`;

export function seat(pose: Pose, width: number, height: number) {
	return ([x, y]: Point): Point => [width / 2 + pose.x + (x - width / 2) * pose.sx, height / 2 + pose.y + (y - height / 2) * pose.sy];
}

export function unseat(pose: Pose, width: number, height: number) {
	return ([x, y]: Point): Point => [width / 2 + (x - width / 2 - pose.x) / pose.sx, height / 2 + (y - height / 2 - pose.y) / pose.sy];
}

export function halo(outline: Point[][], pose: Pose, width: number, height: number) {
	const point = seat(pose, width, height);
	return outline
		.map((polygon) => 'M' + polygon.map((p) => point(p).join(' ')).join('L') + 'Z')
		.join('');
}

export function crest(outline: Point[][], pose: Pose, width: number, height: number): Point {
	const point = seat(pose, width, height);
	const points = outline.flatMap((polygon) => polygon.map(point));
	const xs = points.map(([x]) => x);
	return [(Math.min(...xs) + Math.max(...xs)) / 2, Math.min(...points.map(([, y]) => y))];
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

export function reveal(tiles: PlacedTile[], width: number, height: number, pose: Pose) {
	if (!width || !height) return { grow: [], spread: [] };
	const plane = frame(tiles, width, height);
	const project = projector(plane, width, height);
	const { rows, cols } = grid(plane, width, height);
	const lift = floor(tiles);
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
	const cells = [...distance].map(([cell, d]) => [...cell.split(',').map(Number), d]);
	const layers = Array.from({ length: rings + 1 }, (): number[][] => []);
	const border = cells.filter(([row, col]) => neighbours(row, col).some(([r, c]) => !distance.has(`${r},${c}`)));
	for (const cell of cells) if (cell[2] <= rings) layers[cell[2]].push(cell);
	const spread: Keyframe[] = [{ '--clip': footprint(island(tiles, width, height)), offset: 0, easing: 'step-end' }];
	const steps = Math.min(rings, STEPS);
	for (let step = 1; step <= steps; step++) {
		const k = Math.round((step * rings) / steps);
		const edges = new Map<string, number[][]>();
		for (const [row, col, d] of new Set([...layers[k], ...border])) {
			if (d > k) continue;
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
		spread.push({ '--clip': `path('${shape}')`, offset: 0.5 + (0.5 * step) / (steps + 1), easing: 'step-end' });
	}
	spread.push({ '--clip': 'inset(0)', offset: 1 });
	const grow: Keyframe[] = [
		{ transform: transform(pose), offset: 0, easing: 'cubic-bezier(0.6, 0, 0.3, 1)' },
		{ transform: 'none', offset: 0.5 },
		{ transform: 'none', offset: 1 }
	];
	return { grow, spread };
}
