<script lang="ts">
	import { cubicOut } from 'svelte/easing';
	import { fade } from 'svelte/transition';
	import Dialog from '#lib/Dialog.svelte';
	import building1 from '#lib/assets/images/building1.png';
	import building2 from '#lib/assets/images/building2.png';
	import { BASES } from '#lib/bases.js';
	import { PALETTE, rgb } from '#lib/colors.js';
	import { RING_CHARSETS, RING_COUNT } from '#lib/rings.svelte.js';
	import { params, update } from '#lib/url.svelte.js';
	import { page } from '$app/state';

	let { onback }: { onback: () => void } = $props();

	const SIDES = 48;
	const RADIUS = 1;
	const LEDGE = 0.11;
	const WALL = 0.13;
	const RIM = 0.8;
	const SKY = [16, 6, 4];
	const PERSPECTIVE = 1.2;
	const PITCH = 14;
	const YAW = 20;
	const FORWARD = 0.8;
	const DEPTH = 0.1;
	const RISE = 0.16;
	const NEAR = 0.15;
	const BANDS = 3;
	const CELL = [9, 16];
	const TILE = [20, 8];
	const FONT = "14px 'JetBrains Mono', monospace";
	const CHURN = 0.08;
	const SHARP = 3;
	const FRAMES = 4;
	const STEP = 700;
	const BUILDINGS = [building1, building2];
	const TOWERS = [
		[-3.6, 0.55],
		[-3, 0.75],
		[-2.3, 0.48],
		[-1.4, 0.85],
		[-0.6, 0.52],
		[0.3, 0.7],
		[1, 0.46],
		[1.9, 0.62]
	];
	const ENTER = 450;
	const LEAVE = 250;
	const DIRT = rgb(BASES.dirt.flat);
	const GRAY = rgb('#2a2b2d');
	const SLICE = Math.tan(Math.PI / SIDES) * 2;
	const CORNER = 1 / Math.cos(Math.PI / SIDES);

	const tint = (light: number, depth = 0, alpha = 1) =>
		`rgb(${DIRT.map((c, i) => Math.round((Math.min(1, c * light) * (1 - depth) + GRAY[i] * depth) * 255)).join(' ')} / ${alpha})`;
	const ANGLES = Array.from({ length: SIDES }, (_, k) => (k * 360) / SIDES - 180);
	const radians = (degrees: number) => (degrees * Math.PI) / 180;
	const around = (radius: number) => ANGLES.filter((angle) => radius * Math.cos(radians(angle - YAW)) + FORWARD * Math.cos(radians(YAW)) > NEAR);

	function annulus(outer: number, inner: number) {
		const circle = (r: number) => `M ${outer - r} ${outer} a ${r} ${r} 0 1 0 ${2 * r} 0 a ${r} ${r} 0 1 0 ${-2 * r} 0 Z`;
		return {
			width: `${2 * outer}px`,
			height: `${outer + Math.min(outer, (FORWARD * Math.cos(radians(YAW)) - NEAR) * unit)}px`,
			clip: `path(evenodd, '${circle(outer)} ${circle(inner)}')`,
			transform: `rotateY(${YAW}deg) rotateX(90deg) translate(${-outer}px, ${-outer}px)`
		};
	}

	let innerWidth = $state(window.innerWidth);
	let innerHeight = $state(window.innerHeight);
	const unit = $derived(Math.min(innerHeight, innerWidth * 0.75));
	const perspective = $derived(PERSPECTIVE * innerHeight);

	let textures: string[][] = $state([]);
	let tick = $state(0);

	$effect(() => {
		let cancelled = false;
		const urls: string[] = [];
		document.fonts.load(FONT).then(async () => {
			let light = 0.62;
			const rings = RING_CHARSETS.toReversed().map((set, i) => {
				const pick = () => set[Math.floor(Math.random() * set.length)];
				const base = Array.from({ length: TILE[0] * TILE[1] }, pick);
				const [dark, bright, color] = [tint(light * 0.9, i / RING_COUNT), tint(light * 1.1, i / RING_COUNT), tint(light * 1.8, i / RING_COUNT, 0.5)];
				light *= 0.86;
				const draw = (cells: string[]) => {
					const [width, height] = [TILE[0] * CELL[0], TILE[1] * CELL[1]];
					const canvas = document.createElement('canvas');
					[canvas.width, canvas.height] = [width * SHARP, height * SHARP];
					const context = canvas.getContext('2d')!;
					context.scale(SHARP, SHARP);
					const bands = context.createLinearGradient(0, 0, 0, height);
					for (let k = 0; k <= BANDS * 2; k++) bands.addColorStop(k / (BANDS * 2), k % 2 ? bright : dark);
					context.fillStyle = bands;
					context.fillRect(0, 0, width, height);
					context.font = FONT;
					context.textBaseline = 'top';
					context.fillStyle = color;
					cells.forEach((cell, k) => context.fillText(cell, (k % TILE[0]) * CELL[0], Math.floor(k / TILE[0]) * CELL[1]));
					return new Promise<string>((resolve) =>
						canvas.toBlob((blob) => {
							urls.push(URL.createObjectURL(blob!));
							resolve(`url("${urls.at(-1)}")`);
						})
					);
				};
				return Promise.all([draw(base), ...Array.from({ length: FRAMES }, () => draw(base.map((cell) => (Math.random() < CHURN ? pick() : cell))))]);
			});
			const drawn = (await Promise.all(rings)).toReversed();
			if (!cancelled) textures = drawn;
		});
		return () => {
			cancelled = true;
			urls.forEach((url) => URL.revokeObjectURL(url));
		};
	});

	$effect(() => {
		if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		const id = setInterval(() => tick++, STEP);
		return () => clearInterval(id);
	});

	const phase = (ring: number) => (tick + ring) % 4;
	const overlay = (ring: number) => textures[ring]?.[1 + (Math.floor((tick + ring) / 4) % FRAMES)];

	const terraces = $derived.by(() => {
		let light = 1;
		return Array.from({ length: RING_COUNT }, (_, i) => {
			const outer = (RADIUS - i * LEDGE) * unit;
			const top = i * WALL * unit;
			const wall = 0.62 * light;
			light *= 0.86;
			const ring = RING_COUNT - 1 - i;
			return {
				ring,
				angles: around(RADIUS - i * LEDGE),
				outer,
				inner: outer - LEDGE * unit,
				top,
				bottom: top + WALL * unit,
				dark: tint(wall * 0.9, i / RING_COUNT),
				light: tint(wall * 1.1, i / RING_COUNT),
				ledge: tint(light, (i + 1) / RING_COUNT),
				lit: tint(light * 1.3, (i + 1) / RING_COUNT)
			};
		});
	});

	const rim = $derived(annulus((RADIUS + RIM) * unit, RADIUS * unit - 1));

	let hovered = $state<number | null>(null);
	let pointer = $state({ x: window.innerWidth / 2, y: window.innerHeight / 2 });
	let cardHeight = $state(0);

	const ring = $derived(terraces.find(({ ring }) => String(ring) === params().get('ring'))?.ring);
	const open = $derived(ring !== undefined);
	let selected = $state(0);
	$effect.pre(() => {
		if (ring !== undefined) selected = ring;
	});

	function choose(ring: number) {
		update({ ring }, { state: { ...page.state, ring: true } });
	}

	function dismiss() {
		if (page.state.ring) history.back();
		else update({ ring: null }, { replace: true });
	}

	function onescape(event: KeyboardEvent) {
		if (event.key === 'Escape' && !open) onback();
	}

	function onkeydown(event: KeyboardEvent, ring: number) {
		if (event.key !== 'Enter' && event.key !== ' ') return;
		event.preventDefault();
		choose(ring);
	}

	function drone(depth: number) {
		return `translateZ(${perspective}px) rotateX(${-PITCH}deg) rotateY(${-YAW}deg) translate3d(0, ${-depth * unit}px, ${-FORWARD * unit}px)`;
	}

	const duration = (back: boolean) => (matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : back ? LEAVE : ENTER);

	function fly(_: Element, { back = false } = {}) {
		return { duration: duration(back), easing: cubicOut, css: (t: number) => `transform: ${drone(DEPTH - RISE * (1 - t))}` };
	}
</script>

<svelte:window bind:innerWidth bind:innerHeight onkeydown={onescape} />

<svelte:head>
	<link
		rel="stylesheet"
		href="https://fonts.googleapis.com/css2?family=Share+Tech&family=JetBrains+Mono&display=swap"
	/>
</svelte:head>

<div class="mines" style:--pit={tint(0.45, 0.4)} style:--sky={PALETTE.sky} style:--horizon={PALETTE.horizon} transition:fade={{ duration: duration(false) / 2 }}>
	<div class="stage" style:perspective="{perspective}px" role="presentation" onpointermove={(event) => (pointer = { x: event.clientX, y: event.clientY })}>
		<div class="world" style:transform={drone(DEPTH)} in:fly out:fly={{ back: true }}>
			<div class="sky" style:width="{SKY[0] * unit}px" style:height="{SKY[1] * unit}px" style:transform="rotateY({YAW}deg) translateZ({-SKY[2] * unit}px) translate(-50%, -100%)">
				{#each TOWERS as [x, height], i}
					<img class="tower" src={BUILDINGS[i % BUILDINGS.length]} alt="" style:left="{(SKY[0] / 2 + x) * unit}px" style:height="{height * unit}px" />
				{/each}
			</div>
			<div class="ledge" style:--ledge={tint(1)} style:width={rim.width} style:height={rim.height} style:clip-path={rim.clip} style:transform={rim.transform}></div>
			{#each terraces as terrace, i (terrace.ring)}
				{@const { ring, angles, outer, inner, top, bottom } = terrace}
				{@const last = i === RING_COUNT - 1}
				<div
					class="terrace"
					class:lit={hovered === ring}
					style:--ledge={terrace.ledge}
					style:--lit={terrace.lit}
					style:--dark={terrace.dark}
					style:--light={terrace.light}
					style:--band="{(WALL * unit) / BANDS}px"
					class:shown={phase(ring) < 2}
					style:--glyphs={textures[ring]?.[0]}
					style:--overlay={overlay(ring)}
					role="button"
					tabindex="0"
					aria-label="Ring {ring}"
					onclick={() => choose(ring)}
					onkeydown={(event) => onkeydown(event, ring)}
					onpointerenter={() => (hovered = ring)}
					onpointerleave={() => hovered === ring && (hovered = null)}
					onfocus={(event) => {
						hovered = ring;
						if (event.currentTarget.matches(':focus-visible')) pointer = { x: innerWidth / 2, y: innerHeight / 2 };
					}}
					onblur={() => hovered === ring && (hovered = null)}
				>
					{#each angles as angle}
						<div
							class="wall"
							style:width="{outer * SLICE + 3}px"
							style:background-position-x="{((angle * SIDES) / 360) * outer * SLICE}px"
							style:height="{bottom - top + 1}px"
							style:transform="translateY({top}px) rotateY({angle}deg) translateZ({-outer}px) translate(-50%, 0)"
						></div>
					{/each}
					{#if last}
						<div class="floor" style:width="{outer * CORNER * 2 + 4}px" style:height="{outer * CORNER * 2 + 4}px" style:transform="translateY({bottom}px) rotateX(90deg) translate(-50%, -50%)"></div>
					{:else}
						{@const ledge = annulus(outer * CORNER + 1, inner)}
						<div class="ledge" style:width={ledge.width} style:height={ledge.height} style:clip-path={ledge.clip} style:transform="translateY({bottom}px) {ledge.transform}"></div>
					{/if}
				</div>
			{/each}
		</div>
	</div>

	{#if hovered !== null && !open}
		<div class="card" class:flipped={pointer.x > innerWidth - 280} class:above={pointer.y + 16 + cardHeight > innerHeight} style:translate="{pointer.x}px {pointer.y}px" bind:clientHeight={cardHeight}>
			<h2>Ring {hovered}</h2>
		</div>
	{/if}

	<Dialog bind:open={() => open, (value) => value || !open || dismiss()} title="Ring {selected}" />

	<h1 class="title">The Mine of Projects</h1>

	<button class="back" aria-label="Back" onclick={onback}>
		<svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7" /></svg>
	</button>
</div>

<style>
	.mines {
		--radius: 12px;
		--radius-small: 6px;
		position: fixed;
		inset: 0;
		z-index: 30;
		overflow: hidden;
		background: var(--pit);
		color: var(--text);
		font-family: 'Share Tech', sans-serif;
		color-scheme: dark;
	}

	.mines :global(:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.stage {
		position: absolute;
		inset: 0;
		perspective-origin: calc(200% / 3) 62%;
	}

	.world {
		position: absolute;
		top: 62%;
		left: calc(200% / 3);
		transform-style: preserve-3d;
	}

	.terrace {
		position: absolute;
		transform-style: preserve-3d;
		cursor: pointer;
	}

	.mines .terrace:focus-visible {
		outline: none;
	}

	.sky {
		position: absolute;
		transform-origin: 0 0;
		background: linear-gradient(var(--horizon), var(--sky));
	}

	.tower {
		position: absolute;
		bottom: 0;
		image-rendering: pixelated;
		transform: translateX(-50%);
	}

	.wall,
	.ledge,
	.floor {
		position: absolute;
		transform-origin: 0 0;
		backface-visibility: hidden;
	}

	.wall {
		background-image: var(--glyphs, repeating-linear-gradient(var(--dark), var(--light) calc(var(--band) / 2), var(--dark) var(--band)));
		background-size: auto 100%;
		transform-style: preserve-3d;
	}

	.wall::after {
		content: '';
		position: absolute;
		inset: 0;
		background-image: var(--overlay, none);
		background-size: auto 100%;
		background-position-x: inherit;
		backface-visibility: hidden;
		opacity: 0;
		transform: translateZ(1px);
		transition: opacity 0.7s ease-in-out;
	}

	.shown .wall::after {
		opacity: 1;
	}

	.ledge {
		background: var(--ledge);
	}

	.floor {
		border-radius: 50%;
		background: var(--ledge);
	}

	.lit .ledge,
	.lit .floor {
		background: var(--lit);
	}

	.card {
		position: fixed;
		top: 0;
		left: 0;
		min-width: 120px;
		padding: 10px 14px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		box-shadow: 0 16px 48px var(--shadow);
		font: 13px 'Inter', sans-serif;
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

	.title {
		position: fixed;
		top: 16px;
		left: 50%;
		display: grid;
		place-items: center;
		height: 64px;
		margin: 0;
		padding: 0 28px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		font-size: 32px;
		font-weight: 400;
		white-space: nowrap;
		transform: translateX(-50%);
	}

	@media (max-width: 600px) {
		.title {
			height: 48px;
			padding: 0 14px;
			font-size: 18px;
		}
	}

	.back {
		position: fixed;
		top: 16px;
		left: 16px;
		display: grid;
		place-items: center;
		width: 48px;
		height: 48px;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: inherit;
		cursor: pointer;
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
</style>
