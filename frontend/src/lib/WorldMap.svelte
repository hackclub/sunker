<script lang="ts">
	import { BASES, NAMES } from '#lib/bases.js';
	import { amount, describe } from '#lib/effects.js';
	import { MAP, legible, rgb } from '#lib/colors.js';
	import { CONTOUR, FOG_STRIDE, HIDDEN, MARK, RAMP_STRIDE, ROCK, STRIDE, THIN, TREE_STRIDE, fallback, pack, renderer, sprite, type Renderer } from '#lib/hexes.js';
	import { CORNERS, ENEMIES, WIDTH, center, enemyName, neighbours, place, type Enemy, type Force, type HexMap, type PlacedTile, type Territory } from '#lib/map.js';
	import { BACKGROUND, FLIGHT, LIFT, PERSPECTIVE, TILT, aim, confine, frame, grid, middle, projector, unprojector, type Plane } from '#lib/view.js';
	import { untrack } from 'svelte';
	import { fade } from 'svelte/transition';

	let {
		map,
		territory = [],
		table = false,
		arrows = true,
		detail = [1, 1],
		selectable = null,
		selected = null,
		route = null,
		onmine,
		onpick
	}: {
		map: HexMap;
		territory?: Territory[];
		table?: boolean;
		arrows?: boolean;
		detail?: [number, number];
		selectable?: [number, number][] | null;
		selected?: [number, number] | null;
		route?: [number, number][] | null;
		onmine?: () => void;
		onpick?: (hex: [number, number]) => void;
	} = $props();

	const LEVEL = 0.1;
	const GRID = [...rgb(MAP.background), 0.03];
	const GROVE = 0.4;
	const SCRIBE = 100;
	const ZOOM = 4;
	const STEP = Math.SQRT2;
	const GLIDE = 200;
	const SWEEP = 0.8;
	const ROADS = ['highway', 'street'];
	const roadOf = (tile?: PlacedTile['tile']) => tile?.variants.find((variant) => ROADS.includes(variant)) ?? '';
	const MIDPOINTS = [[2, -2], [4, 0], [2, 2], [-2, 2], [-4, 0], [-2, -2]];

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	let ratio = $state(1);
	let canvas: HTMLCanvasElement;
	let element: HTMLDivElement;
	let painter: Renderer | undefined;
	let restored = $state(0);
	let hovered = $state.raw<PlacedTile | null>(null);
	let region = $state<string | null>(null);
	let pointer = $state({ x: 0, y: 0 });
	let cardHeight = $state(0);
	const choosable = $derived(new Set((selectable ?? []).map(([row, col]) => `${row},${col}`)));
	const pickable = (tile: PlacedTile | null) => Boolean(tile && choosable.has(`${tile.row},${tile.col}`));

	const tiles = $derived(place(map));
	const ceiling = $derived(Math.min(...tiles.map((t) => t.tile.height)) * BACKGROUND);

	let zoom = $state(1);
	let focus = $state<[number, number] | null>(null);
	let dragged = false;
	let grabbing = $state(false);
	let anchor: [number, number] = [0, 0];
	let gliding = 0;
	let still = $state(true);
	const touches = new Map<number, [number, number]>();

	const base = $derived(frame(tiles, innerWidth, innerHeight));
	const plane = $derived(innerWidth && innerHeight ? aim(base, zoom, focus ?? middle(base)) : base);
	const unproject = $derived(unprojector(plane, innerWidth, innerHeight));
	const reach = $derived(Math.max(...tiles.map((t) => t.tile.height)) * LIFT);

	let flown = $state(false);
	$effect(() => {
		if (table) flown = true;
	});

	const members = $derived(new Map((map.regions ?? []).map((r) => [r.name, new Set(r.tiles.map(([row, col]) => `${row},${col}`))])));
	const regions = $derived(
		(map.regions ?? []).flatMap((r) => {
			const spots = r.tiles.flatMap(([row, col]) => placed.get(`${row},${col}`) ?? []);
			if (!spots.length) return [];
			const water = spots.every((t) => t.tile.base === 'ocean');
			const [mx, my] = [spots.reduce((s, t) => s + t.x, 0) / spots.length, spots.reduce((s, t) => s + t.y, 0) / spots.length];
			const lift = (spots.reduce((s, t) => s + t.tile.height, 0) / spots.length) * LIFT;
			const [sxx, syy, sxy] = spots.reduce(([a, b, c], t) => [a + (t.x - mx) ** 2, b + (t.y - my) ** 2, c + (t.x - mx) * (t.y - my)], [0, 0, 0]);
			const limit = water ? 0.8 : 0.45;
			const angle = Math.max(-limit, Math.min(limit, 0.5 * Math.atan2(2 * sxy, sxx - syy)));
			const [ux, uy] = [Math.cos(angle), Math.sin(angle)];
			const along = spots.map((t) => [(t.x - mx) * ux + (t.y - my) * uy, (t.y - my) * ux - (t.x - mx) * uy]);
			const [low, high] = [Math.min(...along.map(([a]) => a)), Math.max(...along.map(([a]) => a))];
			const fit = curve(along);
			const span = high - low + WIDTH;
			const em = water ? 0.8 : 0.95;
			let size = Math.max(0.32, Math.min(0.62, 0.12 * Math.sqrt(spots.length)));
			const width = (text: number) => r.name.length * text * em;
			if (width(size) > span) size = Math.max(0.26, span / (r.name.length * em));
			const half = width(size) / 2;
			const middle = (low + high) / 2;
			const ends = [fit(middle - half), fit(middle + half)];
			const sag = fit(middle) - (ends[0] + ends[1]) / 2;
			const bend = Math.sign(sag || -1) * Math.max(half * 0.12, Math.min(half * 0.3, Math.abs(sag)));
			const offset = (ends[0] + ends[1]) / 2;
			const tilt = Math.tan(Math.max(-limit - angle, Math.min(limit - angle, Math.atan((ends[1] - ends[0]) / (2 * half)))));
			const [cx, cy] = [mx + middle * ux - offset * uy, my + middle * uy + offset * ux];
			const anchor = r.label && placed.get(`${r.label[0]},${r.label[1]}`);
			let points = Array.from({ length: 25 }, (_, i) => {
				const k = (i - 12) / 8;
				const a = middle + k * half;
				const b = offset + tilt * (a - middle) + bend * (1 - k * k);
				return [(mx + a * ux - b * uy - cx) * SCRIBE, (my + a * uy + b * ux - cy) * SCRIBE];
			});
			if (points[0][0] > points[24][0]) points = points.reverse();
			const counts = new Map<string, number>();
			for (const t of spots) {
				const ground = t.tile.variants.includes('mountain') ? MAP.rock : BASES[t.tile.base in BASES ? t.tile.base : 'plain'].flat;
				counts.set(ground, (counts.get(ground) ?? 0) + 1);
			}
			const ground = [...counts].sort((p, q) => q[1] - p[1])[0][0];
			return [
				{
					name: r.name,
					water,
					size: size * SCRIBE,
					ground,
					ink: legible(ground),
					at: anchor ? [anchor.x, anchor.y - lift] : [cx, cy - lift],
					d: 'M' + points.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join('L')
				}
			];
		})
	);

	const labels = $derived.by(() => {
		if (!innerWidth || !innerHeight) return [];
		const project = projector(plane, innerWidth, innerHeight);
		return regions.map((r) => {
			const [ox, oy] = project(r.at[0], r.at[1]);
			const [ax, ay] = project(r.at[0] + 1, r.at[1]);
			const [bx, by] = project(r.at[0], r.at[1] + 1);
			return { ...r, transform: `matrix(${[ax - ox, ay - oy, bx - ox, by - oy].map((v) => (v / SCRIBE).toFixed(4)).join(', ')}, ${ox.toFixed(1)}, ${oy.toFixed(1)})` };
		});
	});

	let moving = $state(false);
	$effect(() => {
		plane;
		moving = true;
		const timer = setTimeout(() => (moving = false), 200);
		return () => clearTimeout(timer);
	});

	const course = $derived.by(() => {
		const spots = (route ?? []).flatMap(([row, col]) => placed.get(`${row},${col}`) ?? []);
		if (spots.length < 2) return null;
		const paved = spots.map(({ tile }) => Boolean(roadOf(tile)));
		const points = spots.map((spot, i) => {
			const [x, y] = paved[i] ? junction(spot) : [spot.x, spot.y];
			return [x, y - spot.tile.height * LIFT];
		});
		const [a, b] = points.slice(-2);
		points[points.length - 1] = [a[0] + (b[0] - a[0]) * 0.45, a[1] + (b[1] - a[1]) * 0.45];
		return { paved, points };
	});

	const trail = $derived.by(() => {
		if (!course || !innerWidth || !innerHeight) return null;
		const { paved, points } = course;
		const project = projector(plane, innerWidth, innerHeight);
		const screen = points.map(([x, y]) => project(x, y));
		const at = (i: number) => screen[Math.max(0, Math.min(screen.length - 1, i))];
		const straight = (i: number) => paved[i] && paved[i + 1];
		const heading = (i: number) => {
			const [from, to] = i > 0 && straight(i - 1) ? [at(i - 1), at(i)] : straight(i) ? [at(i), at(i + 1)] : [at(i - 1), at(i + 1)];
			const length = Math.hypot(to[0] - from[0], to[1] - from[1]) || 1;
			return [(to[0] - from[0]) / length, (to[1] - from[1]) / length];
		};
		const segments = screen.slice(1).map(([x, y], i) => {
			if (straight(i)) return `L${x.toFixed(1)} ${y.toFixed(1)}`;
			const from = at(i);
			const reach = Math.hypot(x - from[0], y - from[1]) / 3;
			const [start, end] = [heading(i), heading(i + 1)];
			return `C${[from[0] + start[0] * reach, from[1] + start[1] * reach, x - end[0] * reach, y - end[1] * reach, x, y].map((v) => v.toFixed(1)).join(' ')}`;
		});
		const [ox, oy] = project(points[0][0], points[0][1]);
		const [ux, uy] = project(points[0][0] + 1, points[0][1]);
		return { d: `M${screen[0].map((v) => v.toFixed(1)).join(' ')}${segments.join('')}`, width: Math.hypot(ux - ox, uy - oy) * 0.1 };
	});

	function curve(points: number[][]) {
		const sums = [0, 0, 0, 0, 0];
		const right = [0, 0, 0];
		for (const [a, b] of points) {
			for (let k = 0; k < 5; k++) sums[k] += a ** k;
			for (let k = 0; k < 3; k++) right[k] += b * a ** k;
		}
		const m = [0, 1, 2].map((i) => [sums[i], sums[i + 1], sums[i + 2], right[i]]);
		for (let i = 0; i < 3; i++) {
			const pivot = m[i][i] || 1e-9;
			for (let j = 0; j < 3; j++) {
				if (j === i) continue;
				const f = m[j][i] / pivot;
				for (let k = i; k < 4; k++) m[j][k] -= f * m[i][k];
			}
		}
		const [c0, c1, c2] = m.map((row, i) => row[3] / (row[i] || 1e-9));
		return (a: number) => c0 + c1 * a + c2 * a * a;
	}

	const placed = $derived(new Map(tiles.map((t) => [`${t.row},${t.col}`, t])));
	const held = $derived(new Map(territory.map((t) => [`${t.row},${t.col}`, t])));
	const occupied = (tile?: PlacedTile) => Boolean(tile && held.get(`${tile.row},${tile.col}`)?.occupied);
	const fog = $derived.by(() => {
		const rings = new Map<string, number>();
		const queue = tiles.filter((tile) => occupied(tile) && neighbours(tile.row, tile.col).some(([r, c]) => placed.has(`${r},${c}`) && !occupied(placed.get(`${r},${c}`))));
		for (const tile of queue) rings.set(`${tile.row},${tile.col}`, 0);
		for (const tile of queue) {
			for (const [r, c] of neighbours(tile.row, tile.col)) {
				const other = placed.get(`${r},${c}`);
				if (!occupied(other) || rings.has(`${r},${c}`)) continue;
				rings.set(`${r},${c}`, rings.get(`${tile.row},${tile.col}`)! + 1);
				queue.push(other!);
			}
		}
		return rings;
	});
	const hidden = (row: number, col: number) => (fog.get(`${row},${col}`) ?? 0) > 1;
	const card = $derived(hovered && (hidden(hovered.row, hovered.col) ? { title: 'Blocked by fog of war', effects: [] } : describe(hovered.tile)));
	const clouds = $derived.by(() => {
		const found = tiles.filter(({ row, col }) => fog.get(`${row},${col}`));
		const data = new Float32Array(found.length * FOG_STRIDE);
		found.forEach(({ tile, row, col, x, y }, i) => data.set([x, y, tile.height * LIFT, hidden(row, col) ? 1 : 0.55], i * FOG_STRIDE));
		return data;
	});
	const sighted = $derived.by(() => {
		if (!hovered || hidden(hovered.row, hovered.col)) return [];
		const force = held.get(`${hovered.row},${hovered.col}`)?.enemies ?? {};
		return ENEMIES.filter((enemy) => force[enemy]).map((enemy) => `${force[enemy]}x ${enemyName(enemy)}`);
	});
	const crowd = (force: Force = {}) => {
		const left = { ...force };
		const found: Enemy[] = [];
		while (found.length < 4 && ENEMIES.some((enemy) => left[enemy])) {
			for (const enemy of ENEMIES) {
				if (!left[enemy] || found.length >= 4) continue;
				found.push(enemy);
				left[enemy]--;
			}
		}
		return found;
	};
	const groups = $derived.by(() => {
		const found = new Map<PlacedTile, Set<PlacedTile>>();
		for (const tile of tiles) {
			if (!tile.tile.variants.includes('mine') || found.has(tile)) continue;
			const group = new Set([tile]);
			for (const { row, col } of group) {
				for (const [r, c] of neighbours(row, col)) {
					const other = placed.get(`${r},${c}`);
					if (other?.tile.variants.includes('mine')) group.add(other);
				}
			}
			for (const member of group) found.set(member, group);
		}
		return found;
	});

	function edges(hex: PlacedTile) {
		const { tile, row, col } = hex;
		const mine = tile.variants.includes('mine');
		return neighbours(row, col).reduce((bits, [r, c], i) => {
			const other = placed.get(`${r},${c}`);
			const step = other ? tile.height - other.tile.height : Infinity;
			const across = mine !== Boolean(other?.tile.variants.includes('mine'));
			const front = occupied(hex) !== occupied(other);
			const kind = step > LEVEL || (mine && across) ? CONTOUR : Math.abs(step) > LEVEL ? 0 : mine || across || front ? HIDDEN : THIN;
			return bits | (kind << (2 * i));
		}, 0);
	}

	function roads(hex: PlacedTile) {
		const { tile, row, col } = hex;
		const around = neighbours(row, col).map(([r, c]) => placed.get(`${r},${c}`));
		const arms = (match: (other: PlacedTile) => boolean, shift: number) => around.reduce((bits, other, i) => (other && match(other) ? bits | (1 << (shift + i)) : bits), 0);
		const road = roadOf(tile);
		let bits = 0;
		if (tile.variants.includes('mine')) bits |= arms((other) => other.tile.variants.includes('mine'), 0);
		if (tile.variants.includes('beach')) bits |= arms((other) => other.tile.base !== tile.base, 6);
		if (road === 'highway') bits |= arms((other) => roadOf(other.tile) === 'highway', 12);
		if (occupied(hex)) bits |= around.reduce((mask, other, i) => (occupied(other) ? mask : mask | (1 << (18 + i))), 0);
		const links = road ? arms((other) => roadOf(other.tile) !== '' && !(road === 'highway' && roadOf(other.tile) === 'highway'), 18) : 0;
		return [bits, links];
	}

	function linked(row: number, col: number) {
		return neighbours(row, col).flatMap(([r, c], i) => (roadOf(placed.get(`${r},${c}`)?.tile) ? [i] : []));
	}

	function through(row: number, col: number) {
		const arms = linked(row, col);
		const stems = arms.filter((i) => i % 3 !== 1);
		const [right, left] = [arms.includes(0) && arms.includes(2), arms.includes(3) && arms.includes(5)];
		if (!arms.includes(1) || !arms.includes(4)) return 0;
		if (stems.length === 1) return MIDPOINTS[stems[0]][0] * 2;
		return right !== left ? (right ? 2 : -2) : 0;
	}

	function sway(row: number, col: number) {
		const arms = linked(row, col);
		const straight = arms.every((i) => !linked(...(neighbours(row, col)[i] as [number, number])).some((k) => k % 3 === 1));
		return !straight ? 0 : arms.join() === '0,2' ? 2 : arms.join() === '3,5' ? -2 : 0;
	}

	function hub(row: number, col: number) {
		return roadOf(placed.get(`${row},${col}`)?.tile) && linked(row, col).length >= 5;
	}

	function node(row: number, col: number) {
		const variant = roadOf(placed.get(`${row},${col}`)?.tile);
		if (!variant) return 0;
		const arms = linked(row, col);
		const around = neighbours(row, col);
		const corner = arms.find((i) => arms.includes((i + 1) % 6) && [[row, col], around[i], around[(i + 1) % 6]].every(([r, c]) => linked(r, c).length >= 3));
		if (corner !== undefined) return (9 + ((corner + 1) % 6)) | (variant === 'highway' ? 128 : 0);
		const spoke = (i: number, k: number) => [hub(row, col), hub(...(around[i] as [number, number])), hub(...(around[k] as [number, number]))].filter(Boolean).length === 1;
		const kind = (i: number) => roadOf(placed.get(`${around[i][0]},${around[i][1]}`)?.tile);
		const pull = (i: number, k: number) => arms.includes(k) && !spoke(i, k) && (variant !== 'highway' || (kind(i) === 'highway' && kind(k) === 'highway'));
		let [x, y] = [0, 0];
		const lone = [];
		for (const i of arms) {
			const [previous, next] = [(i + 5) % 6, (i + 1) % 6];
			if (pull(i, previous) || pull(i, next)) [x, y] = [x + MIDPOINTS[i][0] / 2, y + MIDPOINTS[i][1] / 2];
			else if (i % 3 !== 1 && sway(...(around[i] as [number, number]))) lone.push(i);
		}
		if (through(row, col)) [x, y] = [through(row, col), 0];
		else if (sway(row, col)) x = sway(row, col);
		else if (lone.length === 1) [x, y] = [x + MIDPOINTS[lone[0]][0], arms.some((i) => i % 3 === 1) ? y : 0];
		return (Math.max(-4, Math.min(4, x)) + 4) | ((y + 2) << 4) | (variant === 'highway' ? 128 : 0);
	}

	function nodes({ row, col }: PlacedTile) {
		const tiles = [[row, col], ...neighbours(row, col)] as [number, number][];
		const packed = tiles.map(([r, c]) => node(r, c));
		const hubs = tiles.reduce((bits, [r, c], k) => (hub(r, c) ? bits | (1 << k) : bits), 0);
		const roads = tiles.reduce((bits, [r, c], k) => (roadOf(placed.get(`${r},${c}`)?.tile) ? bits | (1 << k) : bits), 0);
		return [packed[0] | (mixed.has(`${row},${col}`) ? 16 : 0) | (packed[1] << 8) | (packed[2] << 16), packed[3] | (packed[4] << 8) | (packed[5] << 16), packed[6] | (hubs << 8) | (roads << 15)];
	}

	function hash(x: number, y: number, k: number) {
		let h = Math.imul(x, 374761393) + Math.imul(y, 668265263) + Math.imul(k, 1442695041);
		h = Math.imul(h ^ (h >>> 13), 1274126177);
		return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
	}

	function spot(bits: number) {
		return (bits & 15) > 8 ? CORNERS[(bits & 15) - 9] : [((bits & 15) - 4) * 0.2165064, (((bits >> 4) & 7) - 2) * 0.375];
	}

	function junction(tile: PlacedTile) {
		const bits = node(tile.row, tile.col);
		const [x, y] = spot(bits);
		return [tile.x + x, tile.y + y];
	}

	const paths = $derived.by(() => {
		const found: number[][] = [];
		for (const tile of tiles) {
			if (!roadOf(tile.tile)) continue;
			const a = junction(tile);
			const width = roadOf(tile.tile) === 'highway' ? 0.36 : 0.2;
			found.push([...a, ...a, width]);
			for (const [r, c] of neighbours(tile.row, tile.col)) {
				const other = placed.get(`${r},${c}`);
				if (other && roadOf(other.tile)) found.push([...a, ...junction(other), roadOf(tile.tile) === 'highway' && roadOf(other.tile) === 'highway' ? 0.36 : 0.2]);
			}
		}
		return found;
	});

	function clear(x: number, y: number) {
		return paths.every(([ax, ay, bx, by, width]) => {
			const [dx, dy] = [bx - ax, by - ay];
			const t = dx || dy ? Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy))) : 0;
			return Math.hypot(x - ax - dx * t, y - ay - dy * t) > width + 0.15;
		});
	}

	const trees = $derived.by(() => {
		const found: number[][] = [];
		for (const { tile, row, col, x, y } of tiles) {
			if (tile.variants.includes('sunker')) {
				found.push([x, y, tile.height * LIFT, 0.6 + hash(col, row, 5) * 0.2, hash(col, row, 6), ROCK, y]);
				found.push([x + (hash(col, row, 7) - 0.5) * 0.6, y + 0.2 + hash(col, row, 8) * 0.2, tile.height * LIFT, 0.25 + hash(col, row, 9) * 0.1, hash(col, row, 10), ROCK, y]);
			}
			if (tile.variants.includes('mountain')) found.push([x, y + 0.4, tile.height * LIFT, 0.9, hash(col, row, 12), sprite('mountain'), y]);
			if (tile.variants.includes('cactus')) {
				const count = 2 + Math.floor(hash(col, row, 30) * 2);
				for (let k = 0; k < count; k++) {
					const tx = x + (hash(col, row, 31 + k) - 0.5) * 1.1;
					const ty = y + (hash(col, row, 34 + k) - 0.3) * 0.7;
					found.push([tx, ty, tile.height * LIFT, 0.8 + hash(col, row, 37 + k) * 0.5, hash(col, row, 40 + k), sprite('cactus'), y]);
				}
			}
			const enemies = hidden(row, col) ? [] : crowd(held.get(`${row},${col}`)?.enemies);
			enemies.forEach((enemy, k) => {
				const angle = ((k + hash(col, row, 50)) / enemies.length) * Math.PI * 2;
				const spread = enemies.length > 1 ? 0.35 : 0;
				found.push([x + Math.cos(angle) * spread, y + Math.sin(angle) * spread * 0.6, tile.height * LIFT, 0.9 + hash(col, row, 51 + k) * 0.3, 0, sprite(enemy), y]);
			});
			const kinds = tile.variants.includes('buildings') ? [sprite('building1'), sprite('building2')] : tile.variants.includes('houses') ? [sprite('house1'), sprite('house2')] : null;
			if (kinds) {
				const count = 2 + Math.floor(hash(col, row, 13) * 2);
				for (let k = 0; k < count; k++) {
					const along = (k + 0.5) / count - 0.5 + (hash(col, row, 14 + k) - 0.5) * 0.15;
					found.push([x + along * 1.2, y + 0.25 + hash(col, row, 17 + k) * 0.2, tile.height * LIFT, 0.85 + hash(col, row, 20 + k) * 0.3, 0, kinds[hash(col, row, 23 + k) < 0.5 ? 0 : 1], y]);
				}
			}
			if (!tile.variants.includes('forest')) continue;
			for (let row = Math.floor((y - 1) / GROVE); row <= Math.floor((y + 1) / GROVE); row++) {
				for (let col = Math.floor((x - WIDTH / 2) / GROVE); col <= Math.floor((x + WIDTH / 2) / GROVE); col++) {
					if (hash(col, row, 0) > 0.85) continue;
					const tx = (col + 0.15 + hash(col, row, 1) * 0.7) * GROVE;
					const ty = (row + 0.15 + hash(col, row, 2) * 0.7) * GROVE;
					const [dx, dy] = [Math.abs(tx - x), Math.abs(ty - y)];
					if (dx > WIDTH / 2 || dx * 0.5 + dy * WIDTH / 2 > WIDTH / 2) continue;
					found.push([tx, ty, tile.height * LIFT, 0.8 + hash(col, row, 3) * 0.4, hash(col, row, 4), sprite('tree'), y]);
				}
			}
		}
		const kept = found.filter(([x, y]) => clear(x, y));
		const data = new Float32Array(kept.length * TREE_STRIDE);
		kept.sort((a, b) => a[1] - b[1]).forEach((tree, i) => data.set(tree, i * TREE_STRIDE));
		return data;
	});

	const mixed = $derived.by(() => {
		const groups = new Map<string, PlacedTile[]>();
		for (const tile of tiles) {
			if ((node(tile.row, tile.col) & 15) <= 8) continue;
			const key = junction(tile).map((v) => v.toFixed(3)).join();
			groups.set(key, [...(groups.get(key) ?? []), tile]);
		}
		const found = new Map<string, number>();
		for (const group of groups.values()) {
			const heights = group.map((t) => t.tile.height);
			if (Math.min(...heights) === Math.max(...heights)) continue;
			const lift = (heights.reduce((sum, h) => sum + h, 0) / heights.length) * LIFT;
			for (const t of group) found.set(`${t.row},${t.col}`, lift);
		}
		return found;
	});

	function bent(a: PlacedTile, b: PlacedTile) {
		return a.tile.height !== b.tile.height || mixed.has(`${a.row},${a.col}`) || mixed.has(`${b.row},${b.col}`);
	}

	function dropped(a: PlacedTile, b: PlacedTile, c: PlacedTile) {
		const [p, q, r] = [a, b, c].map(junction);
		const hubs = [a, b, c].map((t) => hub(t.row, t.col));
		const gap = (u: number[], v: number[]) => Math.hypot(u[0] - v[0], u[1] - v[1]);
		return hubs.filter(Boolean).length === 1 ? hubs[2] : gap(p, q) > gap(p, r) + 0.01 && gap(p, q) > gap(q, r) + 0.01;
	}

	function links(tile: PlacedTile) {
		if (!roadOf(tile.tile)) return [];
		const around = neighbours(tile.row, tile.col).map(([r, c]) => placed.get(`${r},${c}`));
		return around.flatMap((other, i) => {
			if (!other || !roadOf(other.tile)) return [];
			const sides = [around[(i + 1) % 6], around[(i + 5) % 6]];
			return sides.some((side) => side && roadOf(side.tile) && dropped(tile, other, side)) ? [] : [[i, other] as const];
		});
	}

	function slopes(tile: PlacedTile) {
		if (roadOf(tile.tile)) return links(tile).filter(([, other]) => bent(tile, other));
		const around = neighbours(tile.row, tile.col).map(([r, c]) => placed.get(`${r},${c}`));
		return around.flatMap((other, i) => {
			const next = around[(i + 1) % 6];
			return other && next && roadOf(other.tile) && roadOf(next.tile) && bent(other, next) ? [[i, other] as const] : [];
		});
	}

	const ramps = $derived.by(() => {
		const found: number[] = [];
		for (const tile of tiles) {
			for (const [, other] of links(tile)) {
				const level = other.tile.height === tile.tile.height;
				if (mixed.has(`${tile.row},${tile.col}`) || mixed.has(`${other.row},${other.col}`) || other.tile.height > tile.tile.height || (level && (roadOf(tile.tile) !== 'highway' || roadOf(other.tile) !== 'highway' || other.row * 1000 + other.col < tile.row * 1000 + tile.col))) continue;
				const [a, b] = [junction(tile), junction(other)];
				found.push(...a, tile.tile.height * LIFT, ...b, other.tile.height * LIFT, (roadOf(tile.tile) === 'highway' ? 1 : 0) + (roadOf(other.tile) === 'highway' ? 2 : 0), 0, 0, Math.max(tile.y, other.y), 0, 0);
			}
		}
		const height = (tile: PlacedTile) => mixed.get(`${tile.row},${tile.col}`) ?? tile.tile.height * LIFT;
		const corners = new Map<string, { at: number[]; lift: number; depth: number; arms: [number[], boolean, number][] }>();
		for (const tile of tiles) {
			if ((node(tile.row, tile.col) & 15) <= 8) continue;
			const at = junction(tile);
			const key = at.map((v) => v.toFixed(3)).join();
			const corner = corners.get(key) ?? { at, lift: height(tile), depth: tile.y, arms: [] };
			corners.set(key, corner);
			for (const [, other] of links(tile)) {
				corner.depth = Math.max(corner.depth, tile.y, other.y);
				const b = junction(other);
				if (Math.hypot(b[0] - at[0], b[1] - at[1]) <= 0.01) continue;
				corner.arms.push([b, roadOf(tile.tile) === 'highway' && roadOf(other.tile) === 'highway', height(other) - corner.lift]);
				if (mixed.has(`${tile.row},${tile.col}`)) found.push(...at, corner.lift, ...b, height(other), (roadOf(tile.tile) === 'highway' ? 1 : 0) + (roadOf(other.tile) === 'highway' ? 2 : 0), 0, 0, Math.max(tile.y, other.y), 0, 0);
			}
		}
		for (const { at, lift, depth, arms } of corners.values()) {
			const angle = ([[x, y]]: [number[], boolean, number]) => Math.atan2(y - at[1], x - at[0]);
			arms.sort((a, b) => angle(a) - angle(b));
			arms.forEach((first, k) => {
				const second = arms[(k + 1) % arms.length];
				const gap = (angle(second) - angle(first) + Math.PI * 2) % (Math.PI * 2);
				if (arms.length < 2 || gap >= Math.PI) return;
				const [[ax, ay], [bx, by]] = [first[0], second[0]].map(([x, y]) => [x - at[0], y - at[1]]);
				const det = ax * by - ay * bx;
				const slope = Math.abs(det) < 1e-6 ? [0, 0] : [(first[2] * by - ay * second[2]) / det, (ax * second[2] - first[2] * bx) / det];
				found.push(...at, lift, ...first[0], lift, 4 + (first[1] ? 1 : 0) + (second[1] ? 2 : 0), ...second[0], depth, ...slope);
			});
		}
		return new Float32Array(found);
	});

	const field = $derived.by(() => {
		const { rows, cols } = grid(base, innerWidth, innerHeight, reach, tiles, ZOOM);
		const hexes = [];
		for (let row = rows[0]; row <= rows[1]; row++) {
			for (let col = cols[0]; col <= cols[1]; col++) {
				const here = placed.get(`${row},${col}`);
				hexes.push({ ...center(row, col), placed: here });
			}
		}
		return hexes;
	});

	function pick(screenX: number, screenY: number) {
		const [x, y] = unproject(screenX, screenY);
		for (let i = tiles.length - 1; i >= 0; i--) {
			const tile = tiles[i];
			const dx = Math.abs(x - tile.x);
			const dy = y - tile.y;
			const half = 1 - dx / Math.sqrt(3);
			if (dx <= WIDTH / 2 && dy <= half && dy >= -half - tile.tile.height * LIFT) return tile;
		}
		return null;
	}

	const labelled = (event: Event) => event.target instanceof Element && Boolean(event.target.closest('text'));

	function hold([px, py]: [number, number], [sx, sy]: [number, number], level: number) {
		const next = Math.max(1, Math.min(ZOOM, level));
		const [ux, uy] = unproject(sx, sy);
		const [cx, cy] = middle(plane);
		const k = zoom / next;
		focus = confine(base, tiles, innerWidth, innerHeight, next, [px - (ux - cx) * k, py - (uy - cy) * k]);
		zoom = next;
	}

	function glide(level: number, goal = middle(plane)) {
		cancelAnimationFrame(gliding);
		const [from, origin, start] = [zoom, middle(plane), performance.now()];
		const target = Math.max(1, Math.min(ZOOM, level));
		const step = (now: number) => {
			const t = Math.min(1, (now - start) / GLIDE);
			const e = 1 - (1 - t) ** 3;
			zoom = from + (target - from) * e;
			focus = confine(base, tiles, innerWidth, innerHeight, zoom, [origin[0] + (goal[0] - origin[0]) * e, origin[1] + (goal[1] - origin[1]) * e]);
			if (t < 1) gliding = requestAnimationFrame(step);
		};
		gliding = requestAnimationFrame(step);
	}

	const centroid = (points: [number, number][]): [number, number] => [points.reduce((s, [x]) => s + x, 0) / points.length, points.reduce((s, [, y]) => s + y, 0) / points.length];
	const spread = (points: [number, number][]) => (points.length > 1 ? Math.hypot(points[0][0] - points[1][0], points[0][1] - points[1][1]) : 1);

	function onpointerdown(event: PointerEvent) {
		if (event.button !== 0) return;
		if (!touches.size) {
			dragged = false;
			anchor = [event.clientX, event.clientY];
		}
		touches.set(event.pointerId, [event.clientX, event.clientY]);
	}

	function onpointermove(event: PointerEvent) {
		pointer = { x: event.clientX, y: event.clientY };
		const last = touches.get(event.pointerId);
		if (last) {
			const before = [...touches.values()];
			touches.set(event.pointerId, [event.clientX, event.clientY]);
			const after = [...touches.values()];
			if (!dragged && (touches.size > 1 || Math.hypot(event.clientX - anchor[0], event.clientY - anchor[1]) > 4)) {
				dragged = grabbing = true;
				still = false;
				region = null;
			}
			if (dragged && !element.hasPointerCapture(event.pointerId) && event.buttons) element.setPointerCapture(event.pointerId);
			cancelAnimationFrame(gliding);
			hold(unproject(...centroid(before)), centroid(after), (zoom * spread(after)) / spread(before));
		}
		if (!dragged) still = true;
		hovered = !still || labelled(event) ? null : pick(event.clientX, event.clientY);
	}

	function onpointerup(event: PointerEvent) {
		touches.delete(event.pointerId);
		if (!touches.size) grabbing = false;
	}

	function onclick(event: MouseEvent) {
		if (dragged || labelled(event)) return;
		const tile = pick(event.clientX, event.clientY);
		if (selectable) {
			if (tile && pickable(tile)) onpick?.([tile.row, tile.col]);
		} else if (tile?.tile.variants.includes('mine')) onmine?.();
	}

	$effect(() => {
		if (table) return;
		const wheel = (event: WheelEvent) => {
			event.preventDefault();
			cancelAnimationFrame(gliding);
			const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? innerHeight : 1;
			const [dx, dy] = [event.deltaX * unit, event.deltaY * unit];
			const at: [number, number] = [event.clientX, event.clientY];
			if (event.ctrlKey) hold(unproject(...at), at, zoom * Math.exp(-Math.max(-100, Math.min(100, dy)) * 0.005));
			else hold(unproject(at[0] + dx, at[1] + dy), at, zoom);
			still = false;
			hovered = region = null;
		};
		element.addEventListener('wheel', wheel, { passive: false });
		return () => element.removeEventListener('wheel', wheel);
	});

	$effect(() => {
		if (table || !arrows) return;
		const held = new Set<string>();
		const tapped = new Set<string>();
		let frame = 0;
		let last = 0;
		const step = (now: number) => {
			const span = SWEEP * (held.size ? Math.max(0, Math.min(50, now - last)) : 16);
			last = now;
			const at: [number, number] = [innerWidth / 2, innerHeight / 2];
			const pressed = (key: string) => Number(held.has(key) || tapped.has(key));
			const [dx, dy] = [(pressed('ArrowRight') - pressed('ArrowLeft')) * span, (pressed('ArrowDown') - pressed('ArrowUp')) * span];
			tapped.clear();
			hold(unproject(at[0] + dx, at[1] + dy), at, zoom);
			frame = held.size ? requestAnimationFrame(step) : 0;
		};
		const down = (event: KeyboardEvent) => {
			const target = event.target as Node;
			if (!event.key.startsWith('Arrow') || event.altKey || event.ctrlKey || event.metaKey || (target !== document.body && !element.contains(target))) return;
			event.preventDefault();
			held.add(event.key);
			tapped.add(event.key);
			cancelAnimationFrame(gliding);
			still = false;
			hovered = region = null;
			if (frame) return;
			last = performance.now();
			frame = requestAnimationFrame(step);
		};
		const up = (event: KeyboardEvent) => held.delete(event.key);
		const release = () => held.clear();
		window.addEventListener('keydown', down);
		window.addEventListener('keyup', up);
		window.addEventListener('blur', release);
		return () => {
			cancelAnimationFrame(frame);
			window.removeEventListener('keydown', down);
			window.removeEventListener('keyup', up);
			window.removeEventListener('blur', release);
		};
	});

	$effect(() => {
		if (!innerWidth || !innerHeight) return;
		const fit = base;
		untrack(() => focus && (focus = confine(fit, tiles, innerWidth, innerHeight, zoom, focus)));
	});

	$effect(() => {
		if (!table) return;
		hovered = null;
		region = null;
		touches.clear();
		grabbing = false;
		untrack(() => glide(1, middle(base)));
	});

	$effect(() => {
		const query = matchMedia(`(resolution: ${ratio}dppx)`);
		const update = () => (ratio = devicePixelRatio);
		update();
		query.addEventListener('change', update);
		return () => query.removeEventListener('change', update);
	});

	$effect(() => {
		const lost = (event: Event) => event.preventDefault();
		const restore = () => {
			painter = undefined;
			restored++;
		};
		canvas.addEventListener('webglcontextlost', lost);
		canvas.addEventListener('webglcontextrestored', restore);
		return () => {
			canvas.removeEventListener('webglcontextlost', lost);
			canvas.removeEventListener('webglcontextrestored', restore);
			painter?.destroy();
		};
	});

	const instances = $derived.by(() => {
		const top = ceiling;
		const data = new Float32Array(field.length * STRIDE);
		field.forEach((hex, i) => {
			const o = i * STRIDE;
			data.set([hex.x, hex.y], o);
			if (hex.placed) {
				const { tile } = hex.placed;
				const base = tile.base in BASES ? tile.base : 'plain';
				data.set([tile.height * LIFT, ...rgb(BASES[base].flat), ...rgb(tile.side)], o + 2);
				data[o + 14] = NAMES.indexOf(base) + 1 + pack(tile.variants) * 1000;
				const [arms, links] = roads(hex.placed);
				data[o + 15] = edges(hex.placed) | links;
				data[o + 17] = arms;
				data[o + 16] = slopes(hex.placed).reduce((bits, [i]) => bits | (1 << i), 0);
				if (roadOf(tile) || linked(hex.placed.row, hex.placed.col).length) data.set(nodes(hex.placed), o + 9);
			} else {
				data[o + 2] = top * LIFT;
				data.set(GRID, o + 9);
			}
		});
		return data;
	});

	$effect(() => {
		restored;
		const screen = { width: innerWidth, height: innerHeight, ratio: [ratio * detail[0], ratio * detail[1]] as [number, number] };
		const hexes = field;
		const data = instances;
		const gl = (painter ??= renderer(canvas) ?? fallback(canvas));
		gl.plant(trees);
		gl.shroud(clouds);
		gl.pave(ramps);

		let frame = 0;
		let drawn: PlacedTile | string | null | undefined;
		let shown: Plane | undefined;
		let stamped: [Set<string>, string | null] | undefined;
		const draw = (time: number) => {
			frame = requestAnimationFrame(draw);
			const target = region ?? hovered;
			const view = plane;
			const marks = choosable;
			const chosen = selected && `${selected[0]},${selected[1]}`;
			const restamp = stamped?.[0] !== marks || stamped[1] !== chosen;
			if (!gl.animated && drawn === target && shown === view && !restamp) return;
			if (shown !== view) {
				shown = view;
				const { width, height, scale, x, y } = view;
				gl.resize(screen, { width, height, scale, x, y }, { perspective: PERSPECTIVE, tilt: TILT });
			}
			if (drawn !== target || restamp) {
				drawn = target;
				stamped = [marks, chosen];
				const area = region ? members.get(region) : undefined;
				hexes.forEach((hex, i) => {
					if (!hex.placed) return;
					const focus = area ? area.has(`${hex.placed.row},${hex.placed.col}`) : hovered === hex.placed || Boolean(hovered && groups.get(hovered)?.has(hex.placed));
					const key = `${hex.placed.row},${hex.placed.col}`;
					data[i * STRIDE + 13] = (occupied(hex.placed) ? (focus ? -2 : -1) : focus ? 1 : 0) + (key === chosen ? 2 : marks.has(key) ? 1 : 0) * MARK;
				});
				gl.fill(data);
			}
			gl.draw(time / 1000);
		};
		frame = requestAnimationFrame(draw);
		return () => cancelAnimationFrame(frame);
	});
</script>

<svelte:window bind:innerWidth bind:innerHeight />

<div
	class="map"
	class:table
	class:mine={selectable ? pickable(hovered) : hovered?.tile.variants.includes('mine')}
	class:grabbing
	role="presentation"
	bind:this={element}
	onpointerdown={table ? undefined : onpointerdown}
	onpointermove={table ? undefined : onpointermove}
	onpointerup={onpointerup}
	onpointercancel={onpointerup}
	onpointerleave={(event) => {
		hovered = null;
		onpointerup(event);
	}}
	onclick={table ? undefined : onclick}
>
	<canvas bind:this={canvas}></canvas>
	{#if !table && trail}
		<svg class="trail" width={innerWidth} height={innerHeight}>
			<defs>
				<marker id="trail-arrow" viewBox="0 0 10 10" refX="4" refY="5" markerWidth="3" markerHeight="3" orient="auto">
					<path d="M1 1L9 5L1 9Z" />
				</marker>
			</defs>
			<path class="edge" d={trail.d} stroke-width={trail.width * 1.8} />
			<path d={trail.d} stroke-width={trail.width} marker-end="url(#trail-arrow)" />
		</svg>
	{/if}
	{#if !table && labels.length}
		<div class="labels" class:moving in:fade={{ delay: flown ? FLIGHT * 0.7 : 0, duration: 600 }} out:fade={{ duration: 250 }} style:--active={MAP.line} style:--shade={MAP.background}>
			{#each labels as label, i (label.name)}
				<svg class="label" style:transform={label.transform}>
					<path id="region-label-{i}" d={label.d} fill="none" />
					<text
						class:water={label.water}
						class:active={region === label.name}
						font-size={label.size}
						stroke-width={label.size * 0.2}
						style:--ink={label.ink}
						style:--ground={label.ground}
						role="presentation"
						onpointerenter={() => (region = still ? label.name : null)}
						onpointerleave={() => (region = null)}
					>
						<textPath href="#region-label-{i}" startOffset="50%" text-anchor="middle">{label.name}</textPath>
					</text>
				</svg>
			{/each}
		</div>
	{/if}
	{#if card}
		<div class="card" class:flipped={pointer.x > innerWidth - 280} class:above={pointer.y + 16 + cardHeight > innerHeight} style:left="{pointer.x}px" style:top="{pointer.y}px" bind:clientHeight={cardHeight}>
			<h2>{card.title}</h2>
			{#if card.effects.length}
				<ul>
					{#each card.effects as effect}
						<li class:positive={effect.positive}>
							<span>{effect.name}</span>
							{#if effect.value !== undefined}<span>{amount(effect.value)}</span>{/if}
						</li>
					{/each}
				</ul>
			{/if}
			{#if sighted.length}
				<ul>
					{#each sighted as enemy}
						<li>{enemy}</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</div>
{#if !table}
	<div class="zoom" in:fade={{ delay: flown ? FLIGHT * 0.7 : 0, duration: 600 }} out:fade={{ duration: 250 }}>
		<button aria-label="Zoom out" disabled={zoom <= 1} onclick={() => glide(STEP ** (Math.ceil(Math.log(zoom) / Math.log(STEP) - 0.01) - 1))}>
			<svg viewBox="0 0 24 24"><path d="M6 12h12" /></svg>
		</button>
		<button aria-label="Zoom in" disabled={zoom >= ZOOM} onclick={() => glide(STEP ** (Math.floor(Math.log(zoom) / Math.log(STEP) + 0.01) + 1))}>
			<svg viewBox="0 0 24 24"><path d="M6 12h12M12 6v12" /></svg>
		</button>
	</div>
{/if}

<style>
	.map {
		position: fixed;
		inset: 0;
		overflow: hidden;
		background: var(--background);
	}

	.map:not(.table) {
		touch-action: none;
	}

	.table {
		pointer-events: none;
	}

	.grabbing {
		cursor: grabbing;
	}

	.zoom {
		position: fixed;
		right: 16px;
		bottom: 16px;
		display: flex;
		gap: 8px;
	}

	.zoom button {
		display: grid;
		place-items: center;
		width: 48px;
		height: 48px;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		cursor: pointer;
	}

	.zoom button:hover:not(:disabled) {
		background: var(--hover);
	}

	.zoom button:disabled {
		color: var(--text-muted);
		cursor: default;
	}

	.zoom svg {
		width: 24px;
		height: 24px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2.5;
		stroke-linecap: round;
	}

	.mine {
		cursor: pointer;
	}

	canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.trail {
		position: absolute;
		inset: 0;
		overflow: visible;
		pointer-events: none;
	}

	.trail > path {
		fill: none;
		stroke: #ffffff;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.trail > path.edge {
		stroke: #000000;
	}

	.trail marker path {
		fill: #ffffff;
		stroke: #000000;
		stroke-width: 1.4;
		stroke-linejoin: round;
	}

	.labels {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}

	.label {
		position: absolute;
		top: 0;
		left: 0;
		width: 1px;
		height: 1px;
		overflow: visible;
		transform-origin: 0 0;
	}

	.moving .label {
		will-change: transform;
	}

	.labels text {
		font-family: 'Belgrano', serif;
		letter-spacing: 0.16em;
		text-transform: uppercase;
		fill: var(--ink);
		stroke: var(--ground);
		stroke-linejoin: round;
		paint-order: stroke;
		pointer-events: auto;
		user-select: none;
		transition:
			fill 0.15s,
			stroke 0.15s,
			stroke-opacity 0.15s;
	}

	.labels.moving text {
		stroke-opacity: 0;
	}

	.labels text.water {
		font-style: italic;
		letter-spacing: 0.24em;
		text-transform: none;
	}

	.labels text.active {
		fill: var(--active);
		stroke: var(--shade);
	}

	.card {
		position: absolute;
		min-width: 160px;
		max-width: 260px;
		padding: 10px 14px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		color: var(--text);
		font: 13px 'Inter', sans-serif;
		box-shadow: 0 16px 48px var(--shadow);
		pointer-events: none;
		transform: translate(var(--dx, 16px), var(--dy, 16px));
	}

	.card.flipped {
		--dx: calc(-100% - 16px);
	}

	.card.above {
		--dy: calc(-100% - 16px);
	}

	h2 {
		margin: 0;
		font-size: 16px;
		font-weight: 500;
	}

	ul {
		margin: 8px 0 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		justify-content: space-between;
		gap: 16px;
		color: var(--negative);
	}

	li.positive {
		color: var(--positive);
	}
</style>
