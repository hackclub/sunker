<script lang="ts">
	import SidePanel, { DIALOG_WIDTH, offset } from '#lib/SidePanel.svelte';
	import { PALETTE } from '#lib/colors.js';
	import { image } from '#lib/styles.svelte.js';
	import WorldMap from '#lib/WorldMap.svelte';
	import Mines, { bake } from '#lib/Mines.svelte';
	import { params, update } from '#lib/url.svelte.js';
	import { page } from '$app/state';
	import { place, type PlacedTile } from '#lib/map.js';
	import { FLIGHT, crest, footprint, halo, inside, island, reveal, table as tabled } from '#lib/view.js';
	import Projects from '#lib/hq/Projects.svelte';
	import Propaganda from '#lib/hq/Propaganda.svelte';
	import Help from '#lib/hq/Help.svelte';
	import Chat from '#lib/hq/Chat.svelte';
	import Operations from '#lib/hq/Operations.svelte';
	import { planner } from '#lib/hq/planner.svelte.js';
	import Profile from '#lib/hq/Profile.svelte';
	import type { Component } from 'svelte';

	let { data } = $props();

	type Side = 'left' | 'right';
	type Section = { label: string; icon: string; component: Component; disabled: boolean; dot: boolean };

	const panels: Record<Side, Section[]> = $state({
		left: [
			{ label: 'Projects', icon: 'projects-icon', component: Projects, disabled: false, dot: false },
			{ label: 'Propaganda', icon: 'propaganda-icon', component: Propaganda, disabled: false, dot: false },
			{ label: 'Help', icon: 'help-icon', component: Help, disabled: false, dot: false }
		],
		right: [
			{ label: 'Chat', icon: 'chat-icon', component: Chat, disabled: false, dot: false },
			{ label: 'Operations', icon: 'battle-icon', component: Operations, disabled: false, dot: false },
			{ label: 'Profile', icon: 'profile', component: Profile, disabled: false, dot: false }
		]
	});

	const slug = (section: Section) => section.label.toLowerCase();
	const chosen = (side: Side) => panels[side].findIndex((section) => slug(section) === params().get(side));
	const open: Record<Side, boolean> = $derived({ left: chosen('left') >= 0, right: chosen('right') >= 0 });
	const active: Record<Side, number> = $state({ left: 0, right: 0 });
	$effect.pre(() => {
		for (const side of ['left', 'right'] as const) if (open[side]) active[side] = chosen(side);
	});

	function toggle(side: Side, index: number) {
		update({ [side]: open[side] && active[side] === index ? null : slug(panels[side][index]) });
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape' || mining) return;
		if (wanted) close();
		else if (open.left || open.right) update({ left: null, right: null });
	}

	const PLANKS = ['plank', 'plank2', 'plank3'];
	const BUILDINGS = ['building1', 'building2'];
	const SCALE = 4;
	const WIDTH = 128;
	const HEIGHT = 16;
	const SPACING = 96;
	const MULLION = 3;
	const RAIL = 5;
	const REFRACTION = 8;
	const ANGLE = 40;
	const SMOOTH = false;

	const seed = 0x5eed;

	function pick(row: number, col: number, count: number) {
		const h = Math.imul(seed ^ (row * 73856093) ^ (col * 19349663), 2654435761) >>> 0;
		return ((h ^ (h >>> 16)) >>> 0) % count;
	}

	function random(n: number) {
		const h = Math.imul(seed ^ Math.imul(n + 1, 2246822519), 2654435761) >>> 0;
		return ((h ^ (h >>> 16)) >>> 0) / 2 ** 32;
	}

	let innerWidth = $state(0);
	let innerHeight = $state(0);
	let ratio = $state(1);
	let natural = $state(0);
	const wall = $derived(Math.round((innerHeight * ratio) / 3));
	let floor: HTMLCanvasElement;
	let windows: HTMLCanvasElement;
	let images: HTMLImageElement[] = $state([]);
	let buildings: HTMLImageElement[] = $state([]);

	function load(src: string) {
		const image = new Image();
		image.src = src;
		return image.decode().then(() => image);
	}

	async function settle(sources: string[]) {
		const results = await Promise.allSettled(sources.map(load));
		return results.flatMap((result) => {
			if (result.status === 'fulfilled') return [result.value];
			console.error(result.reason);
			return [];
		});
	}

	const tiles = $derived(place(data.map));
	const pose = $derived(tabled(tiles, innerWidth, innerHeight));
	const outline = $derived(island(tiles, innerWidth, innerHeight));
	const clip = $derived(footprint(outline));
	const border = $derived(halo(outline, tiles, innerWidth, innerHeight));
	const mine = (tile: PlacedTile) => tile.tile.variants.includes('mine');
	const shaft = $derived(outline.filter((_, i) => mine(tiles[i])));
	const seam = $derived(halo(outline, tiles, innerWidth, innerHeight, mine));
	const top = $derived(crest(outline, tiles, innerWidth, innerHeight));
	let hovering = $state<'map' | 'mines' | null>(null);

	function over(event: PointerEvent | MouseEvent) {
		const box = flight.getBoundingClientRect();
		const point: [number, number] = [((event.clientX - box.left) / box.width) * innerWidth, ((event.clientY - box.top) / box.height) * innerHeight];
		return inside(shaft, point) ? 'mines' : inside(outline, point) ? 'map' : null;
	}

	function onpointermove(event: PointerEvent) {
		hovering = !docked || mining ? null : over(event);
	}

	const wanted = $derived(params().has('map'));
	const mining = $derived(params().has('mines'));
	let settled = page.url.searchParams.has('map');
	let docked = $state(!settled);
	let flight: HTMLElement;
	let animations: Animation[] = [];
	let keyframes: ReturnType<typeof reveal> | undefined;

	$effect(() => {
		const size = [tiles, innerWidth, innerHeight] as const;
		keyframes = undefined;
		if (!innerWidth || !innerHeight) return;
		const later = window.requestIdleCallback ?? setTimeout;
		const cancel = window.cancelIdleCallback ?? clearTimeout;
		const id = later(() => (keyframes = reveal(...size)));
		return () => cancel(id);
	});

	function onclick(event: MouseEvent) {
		event.preventDefault();
		if (wanted || mining) return;
		const target = event.detail === 0 ? 'map' : over(event);
		if (target === 'map') update({ map: true }, { state: { map: true } });
		if (target === 'mines') dig();
	}

	function dig() {
		update({ mines: true }, { state: { ...page.state, mines: true } });
	}

	function leave() {
		if (page.state.mines) history.back();
		else update({ mines: null, ring: null }, { replace: true, state: { map: page.state.map } });
	}

	function visit() {
		update({ mines: null, ring: null, left: 'projects' }, { replace: true, state: { map: page.state.map } });
	}

	function close() {
		if (page.state.map) history.back();
		else update({ map: null, operation: null }, { replace: true, state: {} });
	}

	$effect(() => {
		if (params().has('operation') && !wanted) update({ map: true }, { replace: true, state: {} });
	});

	async function fly(opening: boolean) {
		if (opening) docked = false;
		if (animations.length) {
			const now = performance.now();
			animations.forEach((animation) => {
				const start = animation.startTime;
				animation.playbackRate = -animation.playbackRate;
				if (start === null) return;
				const at = Math.max(0, Math.min(FLIGHT, (now - Number(start)) * -animation.playbackRate));
				animation.startTime = now - at / animation.playbackRate;
			});
		} else {
			const { grow, spread } = keyframes ?? reveal(tiles, innerWidth, innerHeight);
			const options: KeyframeAnimationOptions = { duration: FLIGHT, fill: 'both', direction: opening ? 'normal' : 'reverse' };
			animations = [flight.animate(grow, options), flight.animate(spread, options)];
		}
		const current = animations;
		await Promise.all(current.map((animation) => animation.finished));
		if (current !== animations || animations.some((animation) => animation.playState === 'running')) return;
		animations.forEach((animation) => animation.cancel());
		animations = [];
		docked = !settled;
	}

	$effect(() => {
		if (wanted === settled) return;
		settled = wanted;
		fly(wanted);
	});

	$effect(() => {
		const id = (window.requestIdleCallback ?? setTimeout)(() => bake().catch(() => {}));
		return () => (window.cancelIdleCallback ?? clearTimeout)(id);
	});

	$effect(() => {
		settle(PLANKS.map(image)).then((loaded) => (images = loaded));
		settle(BUILDINGS.map(image)).then((loaded) => (buildings = loaded));
	});

	$effect(() => {
		const query = matchMedia(`(resolution: ${ratio}dppx)`);
		const update = () => (ratio = devicePixelRatio);
		update();
		query.addEventListener('change', update);
		return () => query.removeEventListener('change', update);
	});

	const view = $derived.by(() => {
		const dpr = ratio;
		const width = Math.round(innerWidth * dpr);
		const height = Math.round(innerHeight * dpr);
		const focal = height;
		const pitch = (ANGLE * Math.PI) / 180;
		const camera = (focal * Math.sin(pitch)) / (SCALE * dpr);

		const project = (y: number) => {
			const tilt = Math.atan((y + 0.5 - height / 2) / focal);
			const ray = pitch + tilt;
			return { distance: camera / Math.tan(ray), stretch: (camera * Math.cos(tilt)) / Math.sin(ray) / focal };
		};

		const far = project(wall);
		const lift = (y: number) => {
			const tilt = Math.atan((y + 0.5 - height / 2) / focal);
			const ray = pitch + tilt;
			return { z: camera - far.distance * Math.tan(ray), depth: (far.distance * Math.cos(tilt)) / Math.cos(ray) };
		};

		return { width, height, focal, project, far, lift };
	});

	$effect(() => {
		if (!images.length) return;
		const { width, height, project, far } = view;
		const near = project(height - 1);
		const flat = document.createElement('canvas');
		flat.width = Math.ceil(width * far.stretch) + 2;
		flat.height = Math.ceil(far.distance - near.distance) + 2;
		const flatContext = flat.getContext('2d')!;
		for (let row = 0; row * HEIGHT < flat.height; row++) {
			const offset = Math.round(((row % 5) * WIDTH) / 5) - WIDTH;
			for (let col = 0; col * WIDTH + offset < flat.width; col++) {
				flatContext.drawImage(images[pick(row, col, images.length)], col * WIDTH + offset, row * HEIGHT);
			}
		}

		floor.width = width;
		floor.height = height;
		const context = floor.getContext('2d')!;
		context.imageSmoothingEnabled = SMOOTH;
		for (let y = wall; y < height; y++) {
			const { distance, stretch } = project(y);
			const source = SMOOTH ? far.distance - distance : Math.floor(far.distance - distance);
			context.drawImage(flat, flat.width / 2 - (width / 2) * stretch, source, width * stretch, 1, 0, y, width, 1);
		}
	});

	$effect(() => {
		if (!buildings.length) return;
		const { width, focal, lift } = view;
		const snap = (n: number) => (SMOOTH ? n : Math.round(n));
		const span = (left: number, right: number, y: number, rows: number) => {
			const start = snap(left);
			context.fillRect(start, y, snap(right) - start, rows);
		};

		windows.width = width;
		windows.height = wall;
		const context = windows.getContext('2d')!;
		context.imageSmoothingEnabled = SMOOTH;

		const unit = focal / lift(wall / 2).depth;
		const margin = Math.ceil(unit * REFRACTION);
		const backdrop = document.createElement('canvas');
		backdrop.width = width + margin * 2;
		backdrop.height = wall;
		const backdropContext = backdrop.getContext('2d')!;
		backdropContext.imageSmoothingEnabled = SMOOTH;

		const sky = backdropContext.createRadialGradient(backdrop.width / 2, wall * 1.5, 0, backdrop.width / 2, wall * 1.5, wall * 2);
		sky.addColorStop(0, PALETTE.sky);
		sky.addColorStop(0.75, PALETTE.horizon);
		backdropContext.fillStyle = sky;
		backdropContext.fillRect(0, 0, backdrop.width, wall);

		const skyline = [];
		for (let i = 0, x = -unit * 32; x < backdrop.width + unit * 32; i++) {
			const image = buildings[Math.floor(random(i * 3) * buildings.length)];
			const size = unit * (0.5 + random(i * 3 + 1) * 0.7);
			skyline.push({ image, size, x });
			x += image.width * size * (0.4 + random(i * 3 + 2) * 0.5);
		}
		for (const { image, size, x } of skyline.sort((a, b) => a.size - b.size)) {
			const left = snap(x);
			const top = snap(wall - image.height * size);
			backdropContext.drawImage(image, left, top, snap(left + image.width * size) - left, wall - top);
		}

		const pane = (focal / lift(wall).depth) * SPACING;
		for (let k = Math.floor(-width / 2 / pane); width / 2 + k * pane < width; k++) {
			const left = snap(width / 2 + k * pane);
			const right = snap(width / 2 + (k + 1) * pane);
			const shift = snap((random(10000 + k) * 2 - 1) * margin);
			context.drawImage(backdrop, left + margin + shift, 0, right - left, wall, left, 0, right - left, wall);
		}

		context.fillStyle = getComputedStyle(windows).getPropertyValue('--island');
		for (let y = 0; y < wall; y++) {
			const { z } = lift(y);
			if (z < RAIL) context.fillRect(0, y, width, 1);
		}
		const scale = focal / lift(wall).depth;
		const half = width / 2 / scale + MULLION;
		for (let k = Math.ceil(-half / SPACING); k * SPACING <= half; k++) {
			span(width / 2 + (k * SPACING - MULLION / 2) * scale, width / 2 + (k * SPACING + MULLION / 2) * scale, 0, wall);
		}
	});
</script>

<svelte:window bind:innerWidth bind:innerHeight {onkeydown} {onpointermove} />

<svelte:head>
	<title>{mining ? 'Mines' : wanted ? 'Map' : 'HQ'} - Hack Club Sunker</title>
</svelte:head>

<div class="hq">
	<canvas class="floor" class:smooth={SMOOTH} bind:this={floor}></canvas>
	<canvas class="windows" class:smooth={SMOOTH} style:height="{wall / ratio}px" bind:this={windows}></canvas>

	<img class="table" src={image('table')} alt="" style:--px={natural ? `calc(100vw / 2 / ${natural})` : null} bind:naturalWidth={natural} />
	{#if hovering && docked}
		{#if hovering === 'mines'}
			<svg class="outline seam" viewBox="0 0 {innerWidth} {innerHeight}">
				<mask id="seam">
					<rect width={innerWidth} height={innerHeight} fill="white" />
					<path d={seam} fill="black" stroke="none" />
				</mask>
				<path d={seam} stroke-width={innerWidth / 256} mask="url(#seam)" />
			</svg>
		{:else}
			<svg class="outline" viewBox="0 0 {innerWidth} {innerHeight}"><path d={border} stroke-width={innerWidth / 256} /></svg>
		{/if}
		<span class="label" style:top="{top}px">{hovering === 'mines' ? 'Mines' : 'Map'}</span>
	{/if}
	<a class="enter" href="?map" aria-label="Open map" class:hovering {onclick} onfocus={() => (hovering = !docked || mining ? null : 'map')} onblur={() => (hovering = null)}></a>
	<a
		class="dig"
		href="?mines"
		aria-label="Open mines"
		onclick={(event) => {
			event.preventDefault();
			if (!wanted && !mining) dig();
		}}
		onfocus={() => (hovering = !docked || mining ? null : 'mines')}
		onblur={() => (hovering = null)}
	></a>

	{#each ['left', 'right'] as const as side}
		{@const section = panels[side][active[side]]}
		<SidePanel {side} open={open[side]} count={panels[side].length}>
			{#snippet dialog()}
				<header class={side}>
					<h2>{section.label}</h2>
					<button class="close" aria-label="Close" onclick={() => update({ [side]: null })}>
						<svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
							<path d="M2 2L12 12M12 2L2 12" />
						</svg>
					</button>
				</header>
				<div class="body"><section.component /></div>
			{/snippet}
		</SidePanel>
	{/each}

	{#each { length: Math.max(panels.left.length, panels.right.length) }, i}
		{#each ['left', 'right'] as const as side}
			{#if panels[side][i]}
				{@const { label, icon, disabled, dot } = panels[side][i]}
				<button
					class="tab {side}"
					aria-label={label}
					aria-expanded={open[side] && active[side] === i}
					style:top="calc(50% + {offset(panels[side].length, i)}px)"
					style:transform={open[side] ? `translateX(${(side === 'left' ? 1 : -1) * (DIALOG_WIDTH + 8)}px)` : null}
					{disabled}
					onclick={() => toggle(side, i)}
				>
					<img src={image(icon)} alt="" />
					{#if dot}<span class="dot"></span>{/if}
					<span class="tip {side}">{label}</span>
				</button>
			{/if}
		{/each}
	{/each}

	<div class="flight" class:open={wanted} style:transform={wanted ? null : pose} style:--clip={wanted ? null : clip} bind:this={flight}>
		<WorldMap
			map={data.map}
			territory={data.territory}
			table={!wanted}
			arrows={!mining}
			selectable={planner.selectable}
			selected={planner.selected}
			route={planner.path}
			onmine={dig}
			onpick={(hex) => planner.pick?.(hex)}
		/>
		{#if wanted}
			<button class="back" aria-label="Back to HQ" onclick={close}>
				<svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7" /></svg>
			</button>
		{/if}
	</div>

	{#if mining}
		<Mines onback={leave} onproject={visit} />
	{/if}
</div>

<style>
	.hq {
		color: var(--text);
		color-scheme: dark;
		font: 13px 'Inter', sans-serif;
	}

	.hq :global(.button) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 7px 14px;
		border: 1px solid var(--accent);
		border-radius: 6px;
		background: transparent;
		color: var(--accent);
		font: 500 13px 'Inter', sans-serif;
		text-decoration: none;
		white-space: nowrap;
		cursor: pointer;
	}

	.hq :global(.button:not(.primary, .danger):hover:not(:disabled)) {
		background: color-mix(in srgb, var(--accent) 15%, transparent);
	}

	.hq :global(.button.primary) {
		background: var(--accent);
		color: var(--background);
	}

	.hq :global(.button.danger) {
		border-color: var(--negative);
		background: var(--negative);
		color: var(--background);
	}

	.hq :global(:is(.button.primary, .button.danger):hover:not(:disabled)) {
		filter: brightness(1.1);
	}

	.hq :global(.button:disabled) {
		opacity: 0.5;
		cursor: default;
	}

	.hq :global(.input) {
		box-sizing: border-box;
		width: 100%;
		padding: 7px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: 13px 'Inter', sans-serif;
	}

	.hq :global(.input::placeholder) {
		color: var(--text-muted);
	}

	.hq :global(.input:focus) {
		border-color: var(--accent);
		outline: none;
	}

	.hq :global(textarea.input) {
		resize: vertical;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 14px 12px 20px;
	}

	h2 {
		margin: 0;
		font: 24px/24px 'Share Tech', sans-serif;
	}

	.close {
		width: 24px;
		height: 24px;
		border-radius: 6px;
	}

	.close path {
		stroke: var(--text-muted);
		stroke-width: 1.5;
		stroke-linecap: round;
	}

	.body {
		padding: 0 20px 16px;
	}

	button {
		position: relative;
		display: grid;
		place-items: center;
		width: 64px;
		height: 64px;
		padding: 0;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: inherit;
		cursor: pointer;
	}

	button img {
		width: 64px;
		height: 64px;
	}

	.tip,
	.label {
		position: absolute;
		top: 50%;
		padding: 6px 10px;
		border: 1px solid var(--tip-border);
		border-radius: 6px;
		background: var(--island);
		white-space: nowrap;
		transform: translateY(-50%);
		pointer-events: none;
		visibility: hidden;
	}

	.label {
		position: fixed;
		left: 50%;
		z-index: 11;
		transform: translate(-50%, calc(-100% - 16px));
		visibility: visible;
	}

	.tip.left {
		left: calc(100% + 8px);
	}

	.tip.right {
		right: calc(100% + 8px);
	}

	button:hover .tip,
	button:focus-visible .tip {
		visibility: visible;
	}

	button:hover:enabled {
		background: var(--hover);
	}

	button:disabled {
		cursor: default;
	}

	button:disabled img {
		opacity: 0.3;
	}

	.dot {
		position: absolute;
		top: 8px;
		left: 8px;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: var(--dot);
	}

	.tab {
		position: fixed;
		z-index: 21;
		transition: transform 0.25s ease;
	}

	.tab.left {
		left: 0;
	}

	.tab.right {
		right: 0;
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.floor {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		image-rendering: pixelated;
	}

	.windows {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		image-rendering: pixelated;
	}

	.smooth {
		image-rendering: auto;
	}

	img {
		image-rendering: pixelated;
	}

	.outline {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
		fill: var(--outline);
		stroke: var(--outline);
		stroke-linejoin: round;
	}

	.seam {
		z-index: 11;
		fill: none;
	}

	.flight {
		position: fixed;
		inset: 0;
		z-index: 10;
		pointer-events: none;
		will-change: transform;
		clip-path: var(--clip, none);
	}

	.flight.open {
		pointer-events: auto;
	}

	.back {
		position: fixed;
		top: 16px;
		left: 16px;
		width: 48px;
		height: 48px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
	}

	.back:hover {
		background: var(--hover);
	}

	.back svg {
		width: 24px;
		height: 24px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.enter {
		position: fixed;
		inset: 0;
	}

	.enter.hovering {
		cursor: pointer;
	}

	.dig {
		position: fixed;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
	}

	.table {
		position: fixed;
		top: 50%;
		left: 50%;
		width: calc(100vw / 2);
		transform: translate(-50%, -50%);
		filter: drop-shadow(var(--px) 0 var(--table-edge)) drop-shadow(calc(-1 * var(--px)) 0 var(--table-edge))
			drop-shadow(0 var(--px) var(--table-edge)) drop-shadow(0 calc(-1 * var(--px)) var(--table-edge));
	}
</style>
