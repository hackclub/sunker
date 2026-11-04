<script lang="ts">
	import { colour } from '#lib/colors.js';
	import { FACE, HUE, PHOTO, PRIMITIVES, SHAPES, SILHOUETTE, SWATCHES, tint, type Shape } from '#lib/features.js';
	import type { Feature } from '#lib/me.svelte.js';

	let {
		features = $bindable([]),
		hue = $bindable(),
		open = $bindable(false),
		missing = false,
		readonly = false
	}: { features?: Feature[]; hue?: number | null; open?: boolean; missing?: boolean; readonly?: boolean } = $props();

	const SWEEP = `linear-gradient(to right, ${Array.from({ length: 13 }, (_, i) => tint('avatar', i * 30)).join(', ')})`;

	type Drag = { shape: Shape; colour: string; rotation: number; x: number; y: number; dx: number; dy: number; index: number | null };

	let photo: HTMLElement;
	let swatch = $state(SWATCHES[0]);
	let selected = $state<number | null>(null);
	let drag = $state<Drag | null>(null);
	let turning = $state<number | null>(null);
	let unit = $state(1);
	let suppress = false;
	let resetting = $state(false);

	$effect.pre(() => {
		void features;
		selected = null;
	});

	function local(x: number, y: number) {
		const box = photo.getBoundingClientRect();
		return [(x - box.left) / box.width, (y - box.top) / box.height];
	}

	const within = ([x, y]: number[]) => x >= 0 && x <= 1 && y >= 0 && y <= 1;
	const outside = $derived(drag ? !within(local(drag.x + drag.dx, drag.y + drag.dy)) : false);

	function measure() {
		unit = photo.getBoundingClientRect().width / PHOTO.width;
	}

	function grab(event: PointerEvent, shape: Shape) {
		if (event.button !== 0) return;
		event.preventDefault();
		measure();
		selected = null;
		drag = { shape, colour: colour(swatch), rotation: 0, x: event.clientX, y: event.clientY, dx: 0, dy: 0, index: null };
	}

	function hold(event: PointerEvent, index: number) {
		if (event.button !== 0) return;
		event.preventDefault();
		event.stopPropagation();
		suppress = true;
		measure();
		selected = index;
		const feature = features[index];
		const box = photo.getBoundingClientRect();
		drag = {
			shape: SHAPES.get(feature.shape)!,
			colour: feature.colour,
			rotation: feature.rotation,
			x: event.clientX,
			y: event.clientY,
			dx: box.left + feature.x * box.width - event.clientX,
			dy: box.top + feature.y * box.height - event.clientY,
			index
		};
	}

	function turn(event: PointerEvent, index: number) {
		event.preventDefault();
		event.stopPropagation();
		suppress = true;
		turning = index;
	}

	function onpointermove(event: PointerEvent) {
		if (turning !== null) {
			const feature = features[turning];
			const [x, y] = local(event.clientX, event.clientY);
			feature.rotation = Math.round((Math.atan2((y - feature.y) * PHOTO.height, (x - feature.x) * PHOTO.width) * 180) / Math.PI + 90);
			return;
		}
		if (!drag) return;
		drag.x = event.clientX;
		drag.y = event.clientY;
		if (drag.index === null) return;
		const [x, y] = local(drag.x + drag.dx, drag.y + drag.dy);
		features[drag.index].x = x;
		features[drag.index].y = y;
	}

	function onpointerup() {
		turning = null;
		if (!drag) return;
		const point = local(drag.x + drag.dx, drag.y + drag.dy);
		if (drag.index === null && within(point)) {
			features.push({ shape: drag.shape.slug, colour: drag.colour, x: point[0], y: point[1], rotation: 0 });
			selected = features.length - 1;
		} else if (drag.index !== null && !within(point)) remove(drag.index);
		drag = null;
	}

	function add(shape: Shape) {
		features.push({ shape: shape.slug, colour: colour(swatch), x: 0.5, y: 0.4, rotation: 0 });
		selected = features.length - 1;
	}

	function remove(index: number) {
		features.splice(index, 1);
		selected = null;
	}

	function reset() {
		features.length = 0;
		selected = null;
		resetting = false;
	}

	function onclick() {
		if (suppress) {
			suppress = false;
			return;
		}
		if (open && selected !== null) selected = null;
		else open = !open;
	}

	function onkeydown(event: KeyboardEvent, index: number) {
		if (event.key === 'Delete' || event.key === 'Backspace') {
			event.preventDefault();
			remove(index);
		}
	}
</script>

<svelte:window {onpointermove} {onpointerup} />

{#snippet art(shape: Shape, fill: string)}
	{#each shape.strokes as stroke, i (i)}
		<path d={stroke.d} fill={stroke.fill ? fill : 'none'} stroke={stroke.fill ? 'none' : fill} stroke-width={stroke.width} stroke-linecap="round" stroke-linejoin="round" />
	{/each}
{/snippet}

<div class="headshot">
	<div class="photo" class:missing class:readonly role="presentation" bind:this={photo} onpointerdown={() => (suppress = false)}>
		{#if !readonly}<button class="trigger" type="button" aria-label={open ? 'Deselect feature' : 'Edit headshot'} aria-expanded={open} {onclick}></button>{/if}
		<svg viewBox="0 0 {PHOTO.width} {PHOTO.height}" aria-hidden={features.length === 0}>
			<path d={SILHOUETTE} style:fill={tint('passport-silhouette', hue ?? HUE)} />
			{#each features as feature, i (i)}
				{@const shape = SHAPES.get(feature.shape)}
				{#if shape && !(drag?.index === i && outside)}
					<g
						class="feature"
						role="button"
						tabindex={readonly ? -1 : 0}
						aria-label={shape.name}
						transform="translate({feature.x * PHOTO.width} {feature.y * PHOTO.height}) rotate({feature.rotation})"
						onpointerdown={(event) => hold(event, i)}
						onkeydown={(event) => onkeydown(event, i)}
						onfocus={() => (selected = i)}
					>
						<circle class="hit" r={shape.reach} />
						{@render art(shape, feature.colour)}
						{#if selected === i && !drag}
							<circle class="ring" r={shape.reach + 1} />
							<line class="stem" y1={-shape.reach - 1} y2={-shape.reach - 7} />
							<circle class="handle" role="presentation" cy={-shape.reach - 9} r="3" onpointerdown={(event) => turn(event, i)} />
						{/if}
					</g>
				{/if}
			{/each}
		</svg>
	</div>

	{#if open}
		<div class="tray" role="dialog" aria-label="Features" tabindex="-1" onkeydown={(event) => event.key === 'Escape' && (open = false)}>
			<header>
				<p>Drag in features to create your character</p>
				<button class="close" type="button" aria-label="Close features" onclick={() => (open = false)}>
					<svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2 2L12 12M12 2L2 12" /></svg>
				</button>
			</header>
			<div class="grid">
				{#each [...FACE, ...PRIMITIVES] as shape (shape.slug)}
					<button class="item" type="button" aria-label="Add {shape.name.toLowerCase()}" title={shape.name} onpointerdown={(event) => grab(event, shape)} onclick={() => add(shape)}>
						<svg viewBox="{-shape.reach - 1} {-shape.reach - 1} {shape.reach * 2 + 2} {shape.reach * 2 + 2}" aria-hidden="true">{@render art(shape, colour(swatch))}</svg>
					</button>
				{/each}
			</div>
			<label class="tone">
				<span>Character Color</span>
				<input type="range" min="0" max="359" value={hue ?? HUE} style:--sweep={SWEEP} oninput={(event) => (hue = Number(event.currentTarget.value))} />
			</label>
			<div class="swatches" role="radiogroup" aria-label="Colour">
				{#each SWATCHES as key (key)}
					<button class="swatch" type="button" role="radio" aria-checked={swatch === key} aria-label="Colour {key.slice(8)}" style:background={colour(key)} onclick={() => (swatch = key)}></button>
				{/each}
			</div>
			{#if resetting}
				<div class="reset" role="alertdialog" aria-label="Reset headshot">
					<p>Remove every feature from your headshot?</p>
					<div class="actions">
						<button type="button" onclick={() => (resetting = false)}>Cancel</button>
						<button type="button" class="danger" onclick={reset}>Reset</button>
					</div>
				</div>
			{:else}
				<button class="start" type="button" disabled={!features.length} onclick={() => (resetting = true)}>Reset headshot</button>
			{/if}
		</div>
	{/if}
</div>

{#if drag && (drag.index === null || outside)}
	<svg
		class="ghost"
		class:doomed={outside}
		viewBox="{-drag.shape.reach} {-drag.shape.reach} {drag.shape.reach * 2} {drag.shape.reach * 2}"
		style:left="{drag.x + drag.dx}px"
		style:top="{drag.y + drag.dy}px"
		style:width="{drag.shape.reach * 2 * unit}px"
		aria-hidden="true"
	>
		<g transform="rotate({drag.rotation})">{@render art(drag.shape, drag.colour)}</g>
	</svg>
{/if}

<style>
	.headshot {
		position: relative;
	}

	.photo {
		position: relative;
		aspect-ratio: 100 / 128;
		overflow: hidden;
		border: 1px solid var(--passport-guilloche);
		border-radius: 4px;
		background: var(--passport-photo);
	}

	.photo.missing {
		border-color: var(--passport-missing);
		box-shadow: 0 0 0 1px var(--passport-missing);
	}

	.trigger {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		padding: 0;
		border: none;
		background: none;
		cursor: pointer;
	}

	.trigger:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.photo > svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.feature {
		pointer-events: auto;
		cursor: grab;
		touch-action: none;
		outline: none;
	}

	.readonly .feature {
		pointer-events: none;
	}

	.hit {
		fill: transparent;
	}

	.ring {
		fill: none;
		stroke: var(--accent);
		stroke-width: 0.8;
		stroke-dasharray: 2 2;
	}

	.stem {
		stroke: var(--accent);
		stroke-width: 0.8;
	}

	.handle {
		fill: var(--accent);
		stroke: white;
		stroke-width: 0.8;
		cursor: grab;
	}

	.tray {
		position: absolute;
		top: 0;
		left: calc(100% + 12px);
		z-index: 5;
		display: flex;
		flex-direction: column;
		gap: 8px;
		width: 248px;
		padding: 12px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--island);
		box-shadow: 0 12px 32px var(--shadow);
		color: var(--text);
		font: 13px 'Inter', sans-serif;
	}

	header {
		display: flex;
		align-items: flex-start;
		gap: 8px;
	}

	header p {
		flex: 1;
		margin: 0;
		font-weight: 500;
	}

	.close {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		padding: 0;
		border: none;
		border-radius: 4px;
		background: none;
		cursor: pointer;
	}

	.close svg {
		width: 12px;
		height: 12px;
		stroke: var(--text-muted);
		stroke-width: 1.5;
		stroke-linecap: round;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(40px, 1fr));
		gap: 4px;
	}

	.item {
		display: grid;
		place-items: center;
		aspect-ratio: 1;
		padding: 6px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--passport-photo);
		cursor: grab;
		touch-action: none;
	}

	.item:hover {
		border-color: var(--accent);
	}

	.item svg {
		width: 100%;
		height: 100%;
	}

	.tone {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding-top: 8px;
		border-top: 1px solid var(--island-border);
	}

	.tone input {
		width: 100%;
		height: 12px;
		margin: 0;
		border-radius: 6px;
		background: var(--sweep);
		appearance: none;
		cursor: pointer;
	}

	.tone input::-webkit-slider-thumb {
		width: 16px;
		height: 16px;
		border: 2px solid var(--text);
		border-radius: 50%;
		background: var(--island);
		appearance: none;
	}

	.tone input::-moz-range-thumb {
		width: 12px;
		height: 12px;
		border: 2px solid var(--text);
		border-radius: 50%;
		background: var(--island);
	}

	.swatches {
		display: grid;
		grid-template-columns: repeat(10, 1fr);
		gap: 4px;
		padding-top: 8px;
		border-top: 1px solid var(--island-border);
	}

	.swatch {
		aspect-ratio: 1;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 50%;
		cursor: pointer;
	}

	.swatch[aria-checked='true'] {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.start,
	.actions button {
		padding: 6px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: none;
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	.start:hover:enabled,
	.actions button:hover {
		background: var(--hover);
	}

	.start:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.reset {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	.reset p {
		margin: 0;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}

	.actions .danger {
		border-color: var(--negative);
		color: var(--negative);
	}

	.ghost {
		position: fixed;
		z-index: 100;
		transform: translate(-50%, -50%);
		pointer-events: none;
		overflow: visible;
	}

	.doomed {
		opacity: 0.4;
	}

	@media (max-width: 640px) {
		.tray {
			position: fixed;
			inset: auto 0 0;
			width: auto;
			max-height: 50vh;
			overflow-y: auto;
			border-radius: 12px 12px 0 0;
		}
	}
</style>
