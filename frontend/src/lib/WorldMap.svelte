<script lang="ts">
	import { BASES, NAMES } from '#lib/bases.js';
	import { amount, describe } from '#lib/effects.js';
	import { MAP, rgb } from '#lib/colors.js';
	import { CONTOUR, HIDDEN, RAMP_STRIDE, ROCK, STRIDE, THIN, TREE_STRIDE, fallback, pack, renderer, sprite, type Renderer } from '#lib/hexes.js';
	import { CORNERS, WIDTH, center, neighbours, place, type HexMap, type PlacedTile } from '#lib/map.js';
	import { BACKGROUND, LIFT, PERSPECTIVE, TILT, frame, grid, unprojector } from '#lib/view.js';

	let { map, table = false, detail = [1, 1] }: { map: HexMap; table?: boolean; detail?: [number, number] } = $props();

	const LEVEL = 0.1;
	const GRID = [...rgb(MAP.background), 0.03];
	const GROVE = 0.4;
	const ROADS = ['highway', 'street'];
	const roadOf = (tile?: PlacedTile['tile']) => tile?.variants.find((variant) => ROADS.includes(variant)) ?? '';
	const MIDPOINTS = [[2, -2], [4, 0], [2, 2], [-2, 2], [-4, 0], [-2, -2]];

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	let ratio = $state(1);
	let canvas: HTMLCanvasElement;
	let painter: Renderer | undefined;
	let restored = $state(0);
	let hovered = $state.raw<PlacedTile | null>(null);
	let pointer = $state({ x: 0, y: 0 });
	let cardHeight = $state(0);
	const card = $derived(hovered && describe(hovered.tile));

	const tiles = $derived(place(map));
	const ceiling = $derived(Math.min(...tiles.map((t) => t.tile.height)) * BACKGROUND);

	const plane = $derived(frame(tiles, innerWidth, innerHeight));
	const unproject = $derived(unprojector(plane, innerWidth, innerHeight));
	const reach = $derived(Math.max(...tiles.map((t) => t.tile.height)) * LIFT);

	const placed = $derived(new Map(tiles.map((t) => [`${t.row},${t.col}`, t])));
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

	function edges({ tile, row, col }: PlacedTile) {
		const mine = tile.variants.includes('mine');
		return neighbours(row, col).reduce((bits, [r, c], i) => {
			const other = placed.get(`${r},${c}`);
			const step = other ? tile.height - other.tile.height : Infinity;
			const across = mine !== Boolean(other?.tile.variants.includes('mine'));
			const kind = step > LEVEL || (mine && across) ? CONTOUR : Math.abs(step) > LEVEL ? 0 : mine || across ? HIDDEN : THIN;
			return bits | (kind << (2 * i));
		}, 0);
	}

	function roads({ tile, row, col }: PlacedTile) {
		const around = neighbours(row, col).map(([r, c]) => placed.get(`${r},${c}`));
		const arms = (match: (other: PlacedTile) => boolean, shift: number) => around.reduce((bits, other, i) => (other && match(other) ? bits | (1 << (shift + i)) : bits), 0);
		const road = roadOf(tile);
		let bits = 0;
		if (tile.variants.includes('mine')) bits |= arms((other) => other.tile.variants.includes('mine'), 0);
		if (tile.variants.includes('beach')) bits |= arms((other) => other.tile.base !== tile.base, 6);
		if (road === 'highway') bits |= arms((other) => roadOf(other.tile) === 'highway', 12);
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
		return [packed[0] | (packed[1] << 8) | (packed[2] << 16), packed[3] | (packed[4] << 8) | (packed[5] << 16), packed[6] | (hubs << 8) | (roads << 15)];
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
		if (roadOf(tile.tile)) return links(tile).filter(([, other]) => other.tile.height !== tile.tile.height);
		const around = neighbours(tile.row, tile.col).map(([r, c]) => placed.get(`${r},${c}`));
		return around.flatMap((other, i) => {
			const next = around[(i + 1) % 6];
			return other && next && roadOf(other.tile) && roadOf(next.tile) && other.tile.height !== next.tile.height ? [[i, other] as const] : [];
		});
	}

	const ramps = $derived.by(() => {
		const found: number[] = [];
		for (const tile of tiles) {
			for (const [, other] of links(tile)) {
				const level = other.tile.height === tile.tile.height;
				if (other.tile.height > tile.tile.height || (level && (roadOf(tile.tile) !== 'highway' || roadOf(other.tile) !== 'highway' || other.row * 1000 + other.col < tile.row * 1000 + tile.col))) continue;
				const [a, b] = [junction(tile), junction(other)];
				found.push(...a, tile.tile.height * LIFT, ...b, other.tile.height * LIFT, (roadOf(tile.tile) === 'highway' ? 1 : 0) + (roadOf(other.tile) === 'highway' ? 2 : 0), 0, 0, Math.max(tile.y, other.y));
			}
		}
		const corners = new Map<string, { at: number[]; lift: number; depth: number; arms: [number[], boolean][] }>();
		for (const tile of tiles) {
			if ((node(tile.row, tile.col) & 15) <= 8) continue;
			const at = junction(tile);
			const key = at.map((v) => v.toFixed(3)).join();
			const corner = corners.get(key) ?? { at, lift: tile.tile.height * LIFT, depth: tile.y, arms: [] };
			corners.set(key, corner);
			for (const [, other] of links(tile)) {
				corner.depth = Math.max(corner.depth, tile.y, other.y);
				const b = junction(other);
				if (Math.hypot(b[0] - at[0], b[1] - at[1]) > 0.01) corner.arms.push([b, roadOf(tile.tile) === 'highway' && roadOf(other.tile) === 'highway']);
			}
		}
		for (const { at, lift, depth, arms } of corners.values()) {
			const angle = ([[x, y]]: [number[], boolean]) => Math.atan2(y - at[1], x - at[0]);
			arms.sort((a, b) => angle(a) - angle(b));
			arms.forEach((first, k) => {
				const second = arms[(k + 1) % arms.length];
				const gap = (angle(second) - angle(first) + Math.PI * 2) % (Math.PI * 2);
				if (arms.length > 1 && gap < Math.PI) found.push(...at, lift, ...first[0], lift, 4 + (first[1] ? 1 : 0) + (second[1] ? 2 : 0), ...second[0], depth);
			});
		}
		return new Float32Array(found);
	});

	const field = $derived.by(() => {
		const { rows, cols } = grid(plane, innerWidth, innerHeight, reach);
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

	function onpointermove(event: PointerEvent) {
		hovered = pick(event.clientX, event.clientY);
		pointer = { x: event.clientX, y: event.clientY };
	}

	$effect(() => {
		if (table) hovered = null;
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
		const { width, height, scale, x, y } = plane;
		const hexes = field;
		const data = instances;
		const gl = (painter ??= renderer(canvas) ?? fallback(canvas));
		gl.resize(
			{ width: innerWidth, height: innerHeight, ratio: [ratio * detail[0], ratio * detail[1]] },
			{ width, height, scale, x, y },
			{ perspective: PERSPECTIVE, tilt: TILT }
		);
		gl.plant(trees);
		gl.pave(ramps);

		let frame = 0;
		let drawn: PlacedTile | null | undefined;
		const draw = (time: number) => {
			frame = requestAnimationFrame(draw);
			if (!gl.animated && drawn === hovered) return;
			if (drawn !== hovered) {
				drawn = hovered;
				hexes.forEach((hex, i) => {
					if (hex.placed) data[i * STRIDE + 13] = hovered === hex.placed || (hovered && groups.get(hovered)?.has(hex.placed)) ? 1 : 0;
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

<div class="map" class:table role="presentation" onpointermove={table ? undefined : onpointermove} onpointerleave={() => (hovered = null)}>
	<canvas bind:this={canvas}></canvas>
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
		</div>
	{/if}
</div>

<style>
	.map {
		position: fixed;
		inset: 0;
		overflow: hidden;
		background: var(--background);
	}

	.table {
		pointer-events: none;
	}

	canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
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
