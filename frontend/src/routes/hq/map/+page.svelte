<script lang="ts">
	import { STRIDE, fallback, renderer, rgb, type Renderer } from '#lib/hexes.js';
	import { HEIGHT, WIDTH, bounds, center, place, type PlacedTile } from '#lib/map.js';
	import { perlin } from '#lib/noise.js';

	let { data } = $props();

	const PADDING = 4;
	const OVERSCAN = 1.5;
	const TILT = 20;
	const PERSPECTIVE = 1200;
	const LIFT = Math.tan((TILT * Math.PI) / 180);
	const BACKGROUND = 0.9;
	const OUTLINE = [...rgb('#869ecf'), 0.04];
	const GRID = [...rgb('#0e0f10'), 0.03];

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	let ratio = $state(1);
	let canvas: HTMLCanvasElement;
	let gl: Renderer;
	let hovered: PlacedTile | null = null;

	const tiles = $derived(place(data.map));
	const box = $derived(bounds(tiles));
	const ceiling = $derived(Math.min(...tiles.map((t) => t.tile.height)) * BACKGROUND);

	const plane = $derived.by(() => {
		const width = innerWidth * OVERSCAN;
		const height = innerHeight * OVERSCAN;
		const scale = Math.min(innerWidth / (box.width + PADDING * 2), innerHeight / (box.height + PADDING * 2));
		return {
			width,
			height,
			scale,
			x: width / 2 - (box.left + box.width / 2) * scale,
			y: height / 2 - (box.top + box.height / 2) * scale
		};
	});

	const field = $derived.by(() => {
		const { width, height, scale, x, y } = plane;
		const placed = new Map(tiles.map((t) => [`${t.row},${t.col}`, t]));
		const rows = [Math.floor(-y / scale / (HEIGHT * 0.75)) - 1, Math.ceil((height - y) / scale / (HEIGHT * 0.75)) + 2];
		const cols = [Math.floor(-x / scale / WIDTH) - 1, Math.ceil((width - x) / scale / WIDTH) + 1];
		const hexes = [];
		for (let row = rows[0]; row <= rows[1]; row++) {
			for (let col = cols[0]; col <= cols[1]; col++) {
				hexes.push({ ...center(row, col), placed: placed.get(`${row},${col}`) });
			}
		}
		return hexes;
	});

	function pick(screenX: number, screenY: number) {
		const radians = (TILT * Math.PI) / 180;
		const v = screenY - innerHeight / 2;
		const ry = v / (Math.cos(radians) + (v * Math.sin(radians)) / PERSPECTIVE);
		const rx = (screenX - innerWidth / 2) * (1 - (ry * Math.sin(radians)) / PERSPECTIVE);
		const x = (rx + plane.width / 2 - plane.x) / plane.scale;
		const y = (ry + plane.height / 2 - plane.y) / plane.scale;
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
	}

	function noise(x: number, y: number, t: number) {
		const n =
			0.65 * perlin(x * 0.07 + t * 0.012, y * 0.07 + t * 0.006) +
			0.35 * perlin(x * 0.16 - t * 0.008 + 31, y * 0.16 + t * 0.01 + 17);
		return Math.min(1, Math.max(0, n * 0.8 + 0.5));
	}

	$effect(() => {
		const query = matchMedia(`(resolution: ${ratio}dppx)`);
		const update = () => (ratio = devicePixelRatio);
		update();
		query.addEventListener('change', update);
		return () => query.removeEventListener('change', update);
	});

	$effect(() => {
		const { width, height, scale, x, y } = plane;
		const hexes = field;
		const top = ceiling;
		gl ??= renderer(canvas) ?? fallback(canvas);
		gl.resize(
			{ width: innerWidth, height: innerHeight, ratio },
			{ width, height, scale, x, y },
			{ perspective: PERSPECTIVE, tilt: TILT }
		);

		const data = new Float32Array(hexes.length * STRIDE);
		hexes.forEach((hex, i) => {
			const o = i * STRIDE;
			data.set([hex.x, hex.y], o);
			if (hex.placed) {
				const { tile } = hex.placed;
				data.set([tile.height * LIFT, ...rgb(tile.top), ...rgb(tile.base), ...OUTLINE], o + 2);
			} else {
				data.set(GRID, o + 9);
			}
		});

		let frame = 0;
		let drawn: PlacedTile | null | undefined;
		const draw = (time: number) => {
			frame = requestAnimationFrame(draw);
			if (!gl.animated && drawn === hovered) return;
			const still = drawn !== undefined && !gl.animated;
			drawn = hovered;
			const t = gl.animated ? time / 1000 : 0;
			hexes.forEach((hex, i) => {
				const o = i * STRIDE;
				if (hex.placed) {
					data[o + 13] = hovered === hex.placed ? 1 : 0;
					return;
				}
				if (still) return;
				const n = noise(hex.x, hex.y, t);
				const v = (13 + n * 6) / 255;
				const b = v * 0.7;
				data[o + 2] = n * top * LIFT;
				data[o + 3] = v;
				data[o + 4] = v + 1 / 255;
				data[o + 5] = v + 2 / 255;
				data[o + 6] = b;
				data[o + 7] = b;
				data[o + 8] = b + 1 / 255;
			});
			gl.draw(data);
		};
		frame = requestAnimationFrame(draw);
		return () => cancelAnimationFrame(frame);
	});
</script>

<svelte:window bind:innerWidth bind:innerHeight />

<div class="map" role="presentation" {onpointermove} onpointerleave={() => (hovered = null)}>
	<canvas bind:this={canvas}></canvas>
</div>

<style>
	.map {
		position: fixed;
		inset: 0;
		overflow: hidden;
		background: #0e0f10;
	}

	canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
</style>
