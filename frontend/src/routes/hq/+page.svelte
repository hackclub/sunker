<script lang="ts">
	import plank from '#lib/assets/images/plank.png';
	import plank2 from '#lib/assets/images/plank2.png';
	import plank3 from '#lib/assets/images/plank3.png';
	import table from '#lib/assets/images/table.png';
	import building1 from '#lib/assets/images/building1.png';
	import building2 from '#lib/assets/images/building2.png';
	import projectsIcon from '#lib/assets/images/projects-icon.png';
	import propagandaIcon from '#lib/assets/images/propaganda-icon.png';
	import helpIcon from '#lib/assets/images/help-icon.png';
	import chatIcon from '#lib/assets/images/chat-icon.png';
	import battleIcon from '#lib/assets/images/battle-icon.png';
	import profileIcon from '#lib/assets/images/profile.png';
	import xIcon from '#lib/assets/images/x-icon.png';
	import SidePanel from '#lib/SidePanel.svelte';
	import WorldMap from '#lib/WorldMap.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { place } from '#lib/map.js';
	import { FLIGHT, footprint, halo, inside, island, reveal, table as tabled } from '#lib/view.js';
	import Projects from '#lib/hq/Projects.svelte';
	import Propaganda from '#lib/hq/Propaganda.svelte';
	import Help from '#lib/hq/Help.svelte';
	import Chat from '#lib/hq/Chat.svelte';
	import Missions from '#lib/hq/Missions.svelte';
	import Profile from '#lib/hq/Profile.svelte';
	import type { Component } from 'svelte';

	let { data } = $props();

	type Side = 'left' | 'right';
	type Section = { label: string; icon: string; component: Component; disabled: boolean; dot: boolean };

	const panels: Record<Side, Section[]> = $state({
		left: [
			{ label: 'Projects', icon: projectsIcon, component: Projects, disabled: false, dot: false },
			{ label: 'Propaganda', icon: propagandaIcon, component: Propaganda, disabled: false, dot: false },
			{ label: 'Help', icon: helpIcon, component: Help, disabled: false, dot: false }
		],
		right: [
			{ label: 'Chat', icon: chatIcon, component: Chat, disabled: false, dot: false },
			{ label: 'Missions', icon: battleIcon, component: Missions, disabled: false, dot: false },
			{ label: 'Profile', icon: profileIcon, component: Profile, disabled: false, dot: false }
		]
	});

	const open: Record<Side, boolean> = $state({ left: false, right: false });
	const active: Record<Side, number> = $state({ left: 0, right: 0 });

	function toggle(side: Side, index: number) {
		open[side] = !(open[side] && active[side] === index);
		active[side] = index;
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape') return;
		if (wanted) close();
		else open.left = open.right = false;
	}

	const PLANKS = [plank, plank2, plank3];
	const BUILDINGS = [building1, building2];
	const SCALE = 4;
	const WIDTH = 128;
	const HEIGHT = 16;
	const SPACING = 96;
	const MULLION = 3;
	const RAIL = 5;
	const REFRACTION = 8;

	const seed = 0x5eed;

	function pick(row: number, col: number) {
		const h = Math.imul(seed ^ (row * 73856093) ^ (col * 19349663), 2654435761) >>> 0;
		return ((h ^ (h >>> 16)) >>> 0) % PLANKS.length;
	}

	function random(n: number) {
		const h = Math.imul(seed ^ Math.imul(n + 1, 2246822519), 2654435761) >>> 0;
		return ((h ^ (h >>> 16)) >>> 0) / 2 ** 32;
	}

	let angle = $state(40);
	let smooth = $state(false);
	let innerWidth = $state(0);
	let innerHeight = $state(0);
	const wall = $derived(Math.round((innerHeight * devicePixelRatio) / 3));
	let floor: HTMLCanvasElement;
	let windows: HTMLCanvasElement;
	let images: HTMLImageElement[] = $state([]);
	let buildings: HTMLImageElement[] = $state([]);

	function load(src: string) {
		const image = new Image();
		image.src = src;
		return image.decode().then(() => image);
	}

	const tiles = $derived(place(data.map));
	const pose = $derived(tabled(tiles, innerWidth, innerHeight));
	const clip = $derived(footprint(tiles, innerWidth, innerHeight));
	const outline = $derived(island(tiles, innerWidth, innerHeight));
	const border = $derived(halo(tiles, innerWidth, innerHeight));
	let hovering = $state(false);

	function over(event: PointerEvent | MouseEvent) {
		const box = flight.getBoundingClientRect();
		const point: [number, number] = [((event.clientX - box.left) / box.width) * innerWidth, ((event.clientY - box.top) / box.height) * innerHeight];
		return inside(outline, point);
	}

	function onpointermove(event: PointerEvent) {
		hovering = !wanted && over(event);
	}

	const wanted = $derived((page.shallow?.url ?? page.url).searchParams.has('map'));
	let settled = page.url.searchParams.has('map');
	let flight: HTMLElement;
	let animations: Animation[] = [];

	function onclick(event: MouseEvent) {
		event.preventDefault();
		if (!wanted && (event.detail === 0 || over(event))) goto('?map', { state: { map: true }, shallow: true });
	}

	function close() {
		if (page.state.map) history.back();
		else goto(page.url.pathname, { state: {}, shallow: true, replace: true });
	}

	async function fly(opening: boolean) {
		if (animations.length) animations.forEach((animation) => animation.reverse());
		else {
			const { grow, spread } = reveal(tiles, innerWidth, innerHeight);
			const options: KeyframeAnimationOptions = { duration: FLIGHT, fill: 'both', direction: opening ? 'normal' : 'reverse' };
			animations = [flight.animate(grow, options), flight.animate(spread, options)];
		}
		const current = animations;
		await Promise.all(current.map((animation) => animation.finished));
		if (current !== animations || animations.some((animation) => animation.playState === 'running')) return;
		animations.forEach((animation) => animation.cancel());
		animations = [];
	}

	$effect(() => {
		if (wanted === settled) return;
		settled = wanted;
		fly(wanted);
	});

	$effect(() => {
		Promise.all(PLANKS.map(load)).then((loaded) => (images = loaded));
		Promise.all(BUILDINGS.map(load)).then((loaded) => (buildings = loaded));
	});

	const view = $derived.by(() => {
		const dpr = devicePixelRatio;
		const width = Math.round(innerWidth * dpr);
		const height = Math.round(innerHeight * dpr);
		const focal = height;
		const pitch = (angle * Math.PI) / 180;
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
				flatContext.drawImage(images[pick(row, col)], col * WIDTH + offset, row * HEIGHT);
			}
		}

		floor.width = width;
		floor.height = height;
		const context = floor.getContext('2d')!;
		context.imageSmoothingEnabled = smooth;
		for (let y = wall; y < height; y++) {
			const { distance, stretch } = project(y);
			const source = smooth ? far.distance - distance : Math.floor(far.distance - distance);
			context.drawImage(flat, flat.width / 2 - (width / 2) * stretch, source, width * stretch, 1, 0, y, width, 1);
		}
	});

	$effect(() => {
		if (!buildings.length) return;
		const { width, focal, lift } = view;
		const snap = (n: number) => (smooth ? n : Math.round(n));
		const span = (left: number, right: number, y: number, rows: number) => {
			const start = snap(left);
			context.fillRect(start, y, snap(right) - start, rows);
		};

		windows.width = width;
		windows.height = wall;
		const context = windows.getContext('2d')!;
		context.imageSmoothingEnabled = smooth;

		const unit = focal / lift(wall / 2).depth;
		const margin = Math.ceil(unit * REFRACTION);
		const backdrop = document.createElement('canvas');
		backdrop.width = width + margin * 2;
		backdrop.height = wall;
		const backdropContext = backdrop.getContext('2d')!;
		backdropContext.imageSmoothingEnabled = smooth;

		const sky = backdropContext.createRadialGradient(backdrop.width / 2, wall * 1.5, 0, backdrop.width / 2, wall * 1.5, wall * 2);
		sky.addColorStop(0, '#a8d8f0');
		sky.addColorStop(0.75, '#5a8fc8');
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
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500&display=swap" />
</svelte:head>

<div class="hq">
	<canvas class="floor" class:smooth bind:this={floor}></canvas>
	<canvas class="windows" class:smooth style:height="{wall / devicePixelRatio}px" bind:this={windows}></canvas>

	<img class="table" src={table} alt="" />
	{#if hovering && !wanted}
		<svg class="outline" viewBox="0 0 {innerWidth} {innerHeight}"><path d={border} stroke-width={innerWidth / 256} /></svg>
	{/if}
	<a class="enter" href="?map" aria-label="Open map" class:hovering {onclick}></a>

	{#each ['left', 'right'] as const as side}
		{@const section = panels[side][active[side]]}
		<SidePanel {side} open={open[side]}>
			{#each panels[side] as { label, icon, disabled, dot }, i}
				<button
					aria-label={label}
					aria-expanded={open[side] && active[side] === i}
					tabindex={i * 2 + (side === 'left' ? 1 : 2)}
					{disabled}
					onclick={() => toggle(side, i)}
				>
					<img src={icon} alt="" />
					{#if dot}<span class="dot"></span>{/if}
					<span class="tip {side}">{label}</span>
				</button>
			{/each}
			{#snippet dialog()}
				<header class={side}>
					<h2>{section.label}</h2>
					<button class="close" aria-label="Close" onclick={() => (open[side] = false)}>
						<img src={xIcon} alt="" />
					</button>
				</header>
				<div class="body"><section.component /></div>
			{/snippet}
		</SidePanel>
	{/each}

	<div class="flight" class:open={wanted} style:transform={wanted ? null : pose} style:clip-path={wanted ? null : clip} bind:this={flight}>
		<WorldMap map={data.map} table={!wanted} />
		{#if wanted}
			<button class="back" aria-label="Back to HQ" onclick={close}>
				<svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7" /></svg>
			</button>
		{/if}
	</div>
</div>

<style>
	.hq {
		--island: #1e1f22;
		--island-border: #2b2d30;
		--hover: #2b2d30;
		--text: #dfe1e5;
		--accent: #869ecf;
		color: var(--text);
		color-scheme: dark;
		font: 13px 'Inter', sans-serif;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 14px 12px 20px;
	}

	h2 {
		margin: 0;
		font-size: 24px;
		line-height: 24px;
		font-weight: 500;
	}

	.close {
		width: 24px;
		height: 24px;
		border-radius: 6px;
	}

	.close img {
		width: 24px;
		height: 24px;
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

	.tip {
		position: absolute;
		top: 50%;
		padding: 6px 10px;
		border: 1px solid #393b40;
		border-radius: 6px;
		background: var(--island);
		white-space: nowrap;
		transform: translateY(-50%);
		pointer-events: none;
		visibility: hidden;
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
		background: #3574f0;
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
		fill: #fff;
		stroke: #fff;
		stroke-linejoin: round;
	}

	.flight {
		position: fixed;
		inset: 0;
		z-index: 10;
		pointer-events: none;
		will-change: transform;
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
		border: 1px solid #2b2d30;
		border-radius: 6px;
		background: #1e1f22;
	}

	.back:hover {
		background: #2b2d30;
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

	.table {
		position: fixed;
		top: 50%;
		left: 50%;
		width: calc(100vw / 2);
		transform: translate(-50%, -50%);
		--px: calc(100vw / 2 / 512);
		filter: drop-shadow(var(--px) 0 #2e1a10) drop-shadow(calc(-1 * var(--px)) 0 #2e1a10)
			drop-shadow(0 var(--px) #2e1a10) drop-shadow(0 calc(-1 * var(--px)) #2e1a10);
	}
</style>
