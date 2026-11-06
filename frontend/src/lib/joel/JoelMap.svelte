<script lang="ts">
	import { BASES } from '#lib/bases.js';
	import { MAP, PALETTE, colour, css } from '#lib/colors.js';
	import { image, smooth } from '#lib/styles.svelte.js';
	import { CORNERS, ENEMIES, WIDTH, bounds, neighbours, type Force, type PlacedTile } from '#lib/map.js';
	import { forceText, key, placeName, total, type Hex, type OrderKind } from './joel.js';

	export interface Trail {
		kind: OrderKind;
		path: Hex[];
		tone: string;
		faded?: boolean;
		bold?: boolean;
	}

	let {
		tiles,
		names,
		enemies,
		fog = null,
		trails = [],
		draft = null,
		selected = null,
		view = $bindable(null),
		onpick
	}: {
		tiles: PlacedTile[];
		names: Map<string, string>;
		enemies: Map<string, Force>;
		fog?: Map<string, number> | null;
		trails?: Trail[];
		draft?: { path: Hex[]; tone: string } | null;
		selected?: string | null;
		view?: { x: number; y: number; scale: number } | null;
		onpick?: (row: number, col: number) => void;
	} = $props();

	const TINTS: Record<string, number> = { forest: 0.45, farmland: 0.4, houses: 0.45, buildings: 0.55, mountain: 0.55, highway: 0.35, cactus: 0.2 };

	let width = $state(0);
	let height = $state(0);
	let canvas: HTMLCanvasElement;
	let hover = $state<PlacedTile | null>(null);
	let pointer = $state({ x: 0, y: 0 });
	let touched = $state(false);
	let tipWidth = $state(0);
	let tipHeight = $state(0);
	let loaded = $state(0);
	let press: { x: number; y: number; view: { x: number; y: number; scale: number }; moved: boolean; slop: number } | null = null;
	const points = new Map<number, { x: number; y: number }>();
	let pinch: { distance: number; scale: number; at: number[] } | null = null;

	const images = $derived(
		Object.fromEntries(
			ENEMIES.map((enemy) => {
				const sprite = new Image();
				sprite.onload = () => loaded++;
				sprite.src = image(enemy);
				return [enemy, sprite];
			})
		)
	);

	const placed = $derived(new Map(tiles.map((tile) => [key(tile.row, tile.col), tile])));
	const box = $derived(bounds(tiles));
	const occupied = (row: number, col: number) => total(enemies.get(key(row, col))) > 0;

	const fitted = () => Math.min(width / box.width, height / box.height) * 0.94;
	const zoom = (scale: number) => Math.min(120, Math.max(width && height && tiles.length ? Math.min(4, fitted()) : 4, scale));

	export function fit() {
		if (!width || !height || !tiles.length) return;
		view = { x: box.left + box.width / 2, y: box.top + box.height / 2, scale: fitted() };
	}

	$effect(() => {
		if (!view && width && height && tiles.length) fit();
	});

	const screen = (x: number, y: number) => [(x - view!.x) * view!.scale + width / 2, (y - view!.y) * view!.scale + height / 2];
	const world = (x: number, y: number) => [(x - width / 2) / view!.scale + view!.x, (y - height / 2) / view!.scale + view!.y];

	function pick(x: number, y: number) {
		if (!view) return null;
		const [wx, wy] = world(x, y);
		let best: PlacedTile | null = null;
		let distance = Infinity;
		for (const tile of tiles) {
			const d = Math.hypot(tile.x - wx, tile.y - wy);
			if (d < distance) [best, distance] = [tile, d];
		}
		return distance < WIDTH / 2 ? best : null;
	}

	function hex(context: CanvasRenderingContext2D, x: number, y: number, s: number) {
		context.beginPath();
		CORNERS.forEach(([cx, cy], i) => (i ? context.lineTo(x + cx * s, y + cy * s) : context.moveTo(x + cx * s, y + cy * s)));
		context.closePath();
	}

	function edge(context: CanvasRenderingContext2D, x: number, y: number, s: number, i: number) {
		const [a, b] = [CORNERS[(i + 5) % 6], CORNERS[i]];
		context.moveTo(x + a[0] * s, y + a[1] * s);
		context.lineTo(x + b[0] * s, y + b[1] * s);
	}

	function arrow(context: CanvasRenderingContext2D, from: number[], to: number[], size: number) {
		const angle = Math.atan2(to[1] - from[1], to[0] - from[0]);
		context.beginPath();
		context.moveTo(to[0], to[1]);
		context.lineTo(to[0] - Math.cos(angle - 0.45) * size, to[1] - Math.sin(angle - 0.45) * size);
		context.lineTo(to[0] - Math.cos(angle + 0.45) * size, to[1] - Math.sin(angle + 0.45) * size);
		context.closePath();
		context.fill();
	}

	function trace(context: CanvasRenderingContext2D, path: Hex[], tone: string, kind: OrderKind, line: number, alpha: number, dashed = false) {
		const s = view!.scale;
		const points = path.flatMap(([row, col]) => {
			const tile = placed.get(key(row, col));
			return tile ? [screen(tile.x, tile.y)] : [];
		});
		if (!points.length) return;
		context.globalAlpha = alpha;
		context.strokeStyle = tone;
		context.fillStyle = tone;
		context.lineWidth = line;
		context.lineCap = 'round';
		context.lineJoin = 'round';
		if (kind !== 'move') {
			const [x, y] = points[0];
			context.beginPath();
			context.arc(x, y, s * 0.42, 0, Math.PI * 2);
			context.stroke();
			context.beginPath();
			context.moveTo(x - s * 0.2, y);
			context.lineTo(x + s * 0.2, y);
			if (kind === 'spawn') {
				context.moveTo(x, y - s * 0.2);
				context.lineTo(x, y + s * 0.2);
			}
			context.stroke();
			context.globalAlpha = 1;
			return;
		}
		context.setLineDash(dashed ? [line * 2, line * 1.6] : []);
		context.beginPath();
		points.forEach(([x, y], i) => (i ? context.lineTo(x, y) : context.moveTo(x, y)));
		context.stroke();
		context.setLineDash([]);
		context.beginPath();
		context.arc(points[0][0], points[0][1], line * 1.4, 0, Math.PI * 2);
		context.fill();
		if (dashed) for (const [x, y] of points.slice(1)) context.fillRect(x - line, y - line, line * 2, line * 2);
		if (points.length > 1) arrow(context, points.at(-2)!, points.at(-1)!, Math.max(10, line * 3.4));
		context.globalAlpha = 1;
	}

	$effect(() => {
		void loaded;
		if (!canvas || !view || !width || !height) return;
		const ratio = devicePixelRatio;
		canvas.width = width * ratio;
		canvas.height = height * ratio;
		const context = canvas.getContext('2d')!;
		context.setTransform(ratio, 0, 0, ratio, 0, 0);
		context.fillStyle = PALETTE.background;
		context.fillRect(0, 0, width, height);
		const s = view.scale;

		for (const { tile, row, col, x, y } of tiles) {
			const [sx, sy] = screen(x, y);
			if (sx < -s * 2 || sy < -s * 2 || sx > width + s * 2 || sy > height + s * 2) continue;
			hex(context, sx, sy, s);
			context.fillStyle = BASES[tile.base]?.flat ?? BASES.plain.flat;
			context.fill();
			for (const variant of tile.variants) {
				const tint = TINTS[variant];
				if (!tint) continue;
				context.fillStyle = css(colour(`joel-${variant}`), tint);
				context.fill();
			}
			if (tile.base !== 'ocean' && tile.height < 1.5) {
				context.fillStyle = css(MAP.shadow, Math.max(0, (1.2 - tile.height) * 0.22));
				context.fill();
			}
			context.strokeStyle = css(PALETTE.background, 0.35);
			context.lineWidth = 1;
			context.stroke();
			if (occupied(row, col)) {
				context.fillStyle = css(MAP.hostile, 0.35);
				context.fill();
			}
			const ring = fog?.get(key(row, col));
			if (ring) {
				context.fillStyle = css(MAP.fog, ring > 1 ? 0.8 : 0.4);
				context.fill();
			}
		}

		context.lineCap = 'round';
		for (const { row, col, x, y } of tiles) {
			const [sx, sy] = screen(x, y);
			const mine = names.get(key(row, col));
			const hostile = occupied(row, col);
			neighbours(row, col).forEach(([r, c], i) => {
				const other = placed.get(key(r, c));
				if (hostile && !(other && occupied(r, c))) {
					context.beginPath();
					edge(context, sx, sy, s, i);
					context.strokeStyle = MAP.hostileBorder;
					context.lineWidth = 2.5;
					context.stroke();
				}
				if ((mine || names.get(key(r, c))) && names.get(key(r, c)) !== mine && (other || mine)) {
					context.beginPath();
					edge(context, sx, sy, s, i);
					context.strokeStyle = css(PALETTE.text, 0.85);
					context.lineWidth = 1.5;
					context.setLineDash([4, 3]);
					context.stroke();
					context.setLineDash([]);
				}
			});
		}

		if (s > 9) {
			const groups = new Map<string, number[][]>();
			for (const { row, col, x, y } of tiles) {
				const name = names.get(key(row, col));
				if (name) groups.set(name, [...(groups.get(name) ?? []), [x, y]]);
			}
			context.font = `600 ${Math.min(16, Math.max(11, s * 0.5))}px Inter, sans-serif`;
			context.textAlign = 'center';
			context.textBaseline = 'middle';
			for (const [name, points] of groups) {
				const [sx, sy] = screen(points.reduce((sum, p) => sum + p[0], 0) / points.length, points.reduce((sum, p) => sum + p[1], 0) / points.length);
				context.lineWidth = 3;
				context.strokeStyle = css(PALETTE.background, 0.8);
				context.strokeText(name, sx, sy);
				context.fillStyle = PALETTE.text;
				context.fillText(name, sx, sy);
			}
		}

		for (const trail of trails) trace(context, trail.path, trail.tone, trail.kind, trail.bold ? 5 : 3, trail.faded ? 0.35 : 0.95);
		if (draft?.path.length) trace(context, draft.path, draft.tone, 'move', 4, 1, true);

		for (const { row, col, x, y } of tiles) {
			const force = enemies.get(key(row, col));
			if (!total(force)) continue;
			const [sx, sy] = screen(x, y);
			const kinds = ENEMIES.filter((enemy) => force![enemy]);
			if (s >= 26) {
				const icon = s * 0.5;
				const start = sx - ((kinds.length - 1) * icon * 0.95) / 2;
				context.font = `600 ${Math.max(9, s * 0.26)}px Inter, sans-serif`;
				context.textAlign = 'center';
				context.textBaseline = 'middle';
				kinds.forEach((enemy, i) => {
					const cx = start + i * icon * 0.95;
					const image = images[enemy];
					if (image.complete && image.naturalWidth) {
						const h = icon;
						const w = (image.naturalWidth / image.naturalHeight) * h;
						context.imageSmoothingEnabled = smooth(enemy);
						context.drawImage(image, cx - w / 2, sy - h * 0.75, w, h);
					}
					context.lineWidth = 3;
					context.strokeStyle = PALETTE.background;
					context.strokeText(String(force![enemy]), cx, sy + icon * 0.48);
					context.fillStyle = colour('joel-label');
					context.fillText(String(force![enemy]), cx, sy + icon * 0.48);
				});
			} else {
				context.beginPath();
				context.arc(sx, sy, Math.max(5, s * 0.36), 0, Math.PI * 2);
				context.fillStyle = MAP.hostile;
				context.fill();
				context.font = `600 ${Math.max(8, s * 0.38)}px Inter, sans-serif`;
				context.textAlign = 'center';
				context.textBaseline = 'middle';
				context.fillStyle = colour('joel-label');
				context.fillText(String(total(force)), sx, sy + 0.5);
			}
		}

		const outline = (tile: PlacedTile | null | undefined, color: string, line: number) => {
			if (!tile) return;
			const [sx, sy] = screen(tile.x, tile.y);
			hex(context, sx, sy, s * 0.96);
			context.strokeStyle = color;
			context.lineWidth = line;
			context.stroke();
		};
		outline(hover, css(colour('joel-hover'), 0.8), 1.5);
		outline(selected ? placed.get(selected) : null, PALETTE.accent, 3);
	});

	function middle() {
		const [a, b] = [...points.values()];
		const rect = canvas.getBoundingClientRect();
		return { x: (a.x + b.x) / 2 - rect.left, y: (a.y + b.y) / 2 - rect.top, distance: Math.hypot(a.x - b.x, a.y - b.y) || 1 };
	}

	function down(event: PointerEvent) {
		if (!view) return;
		canvas.setPointerCapture(event.pointerId);
		points.set(event.pointerId, { x: event.clientX, y: event.clientY });
		if (event.pointerType === 'touch') hover = null;
		if (points.size === 2) {
			const { x, y, distance } = middle();
			pinch = { distance, scale: view.scale, at: world(x, y) };
			press = null;
			hover = null;
		} else if (points.size === 1) {
			press = { x: event.clientX, y: event.clientY, view: { ...view }, moved: false, slop: event.pointerType === 'mouse' ? 4 : 10 };
		}
	}

	function move(event: PointerEvent) {
		const rect = canvas.getBoundingClientRect();
		pointer = { x: event.clientX - rect.left, y: event.clientY - rect.top };
		if (points.has(event.pointerId)) points.set(event.pointerId, { x: event.clientX, y: event.clientY });
		if (pinch && points.size === 2) {
			const { x, y, distance } = middle();
			const scale = zoom((pinch.scale * distance) / pinch.distance);
			view = { scale, x: pinch.at[0] - (x - width / 2) / scale, y: pinch.at[1] - (y - height / 2) / scale };
			return;
		}
		if (press) {
			const dx = event.clientX - press.x;
			const dy = event.clientY - press.y;
			if (Math.hypot(dx, dy) > press.slop) press.moved = true;
			if (press.moved) view = { ...press.view, x: press.view.x - dx / press.view.scale, y: press.view.y - dy / press.view.scale };
		}
		hover = press?.moved || event.pointerType === 'touch' ? null : pick(pointer.x, pointer.y);
	}

	function cancel(event: PointerEvent) {
		points.delete(event.pointerId);
		pinch = null;
		press = null;
	}

	function up(event: PointerEvent) {
		points.delete(event.pointerId);
		if (pinch) {
			pinch = null;
			const [rest] = [...points.values()];
			press = rest && view ? { x: rest.x, y: rest.y, view: { ...view }, moved: true, slop: 10 } : null;
			return;
		}
		if (press && !press.moved) {
			const rect = canvas.getBoundingClientRect();
			const tile = pick(event.clientX - rect.left, event.clientY - rect.top);
			touched = event.pointerType === 'touch';
			if (touched) {
				pointer = { x: event.clientX - rect.left, y: event.clientY - rect.top };
				hover = tile;
			}
			if (tile) onpick?.(tile.row, tile.col);
		}
		press = null;
	}

	function wheel(event: WheelEvent) {
		if (!view) return;
		event.preventDefault();
		const rect = canvas.getBoundingClientRect();
		const [px, py] = [event.clientX - rect.left, event.clientY - rect.top];
		const [wx, wy] = world(px, py);
		const scale = zoom(view.scale * Math.exp(-event.deltaY * 0.0015));
		view = { scale, x: wx - (px - width / 2) / scale, y: wy - (py - height / 2) / scale };
	}

	const tipAt = $derived.by(() => {
		const above = pointer.y - tipHeight - 28;
		const left = touched ? pointer.x - tipWidth / 2 : pointer.x + 14;
		const top = touched ? (above >= 4 ? above : pointer.y + 28) : pointer.y + 14;
		return { left: Math.max(4, Math.min(width - tipWidth - 4, left)), top: Math.max(4, Math.min(height - tipHeight - 4, top)) };
	});

	const tip = $derived.by(() => {
		if (!hover) return null;
		const at: Hex = [hover.row, hover.col];
		const force = enemies.get(key(hover.row, hover.col));
		const ring = fog?.get(key(hover.row, hover.col));
		return {
			place: placeName(names, at),
			coords: names.has(key(...at)) ? `${hover.row}, ${hover.col}` : '',
			enemies: total(force) ? forceText(force) : '',
			fog: ring === undefined ? '' : ring > 1 ? 'Hidden in fog' : ring === 1 ? 'Misty' : 'Visible to players'
		};
	});
</script>

<div class="map" bind:clientWidth={width} bind:clientHeight={height}>
	<canvas
		bind:this={canvas}
		style:width="{width}px"
		style:height="{height}px"
		onpointerdown={down}
		onpointermove={move}
		onpointerup={up}
		onpointercancel={cancel}
		onpointerleave={(event) => event.pointerType !== 'touch' && (hover = null)}
		onwheel={wheel}
	></canvas>
	<button class="fit" onclick={fit}>Fit</button>
	{#if tip}
		<div class="tip" bind:clientWidth={tipWidth} bind:clientHeight={tipHeight} style:left="{tipAt.left}px" style:top="{tipAt.top}px">
			<strong>{tip.place}</strong>
			{#if tip.coords}<span class="muted">{tip.coords}</span>{/if}
			{#if tip.enemies}<span class="hostile">{tip.enemies}</span>{/if}
			{#if tip.fog}<span class="muted">{tip.fog}</span>{/if}
		</div>
	{/if}
</div>

<style>
	.map {
		position: relative;
		width: 100%;
		height: 100%;
		overflow: hidden;
		border-radius: 12px;
	}

	canvas {
		display: block;
		cursor: crosshair;
		touch-action: none;
	}

	.fit {
		position: absolute;
		right: 12px;
		bottom: 12px;
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.fit:hover {
		background: var(--hover);
	}

	.tip {
		position: absolute;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 6px 10px;
		border: 1px solid var(--tip-border);
		border-radius: 6px;
		background: var(--island);
		font-size: 12px;
		pointer-events: none;
		white-space: nowrap;
	}

	.muted {
		color: var(--text-muted);
	}

	.hostile {
		color: var(--negative);
	}

	@media (pointer: coarse) {
		canvas {
			-webkit-tap-highlight-color: transparent;
			-webkit-touch-callout: none;
			user-select: none;
		}

		.fit {
			min-width: 44px;
			min-height: 44px;
		}
	}
</style>
