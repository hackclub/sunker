<script lang="ts">
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import Dialog from '#lib/Dialog.svelte';
	import { params, update } from '#lib/url.svelte.js';
	import { page } from '$app/state';
	import MatrixBackground from '#lib/MatrixBackground.svelte';
	import { CENTER_OFFSET, RING_COUNT, RING_WIDTH, VIEW_HEIGHT, ringsState } from '#lib/rings.svelte.js';

	let { onback }: { onback: () => void } = $props();

	const rings = Array.from({ length: RING_COUNT }, (_, k) => k);

	const ring = $derived(rings.find((k) => String(k) === params().get('ring')));
	const open = $derived(ring !== undefined);
	let selected = $state(0);
	$effect.pre(() => {
		if (ring !== undefined) selected = ring;
	});

	ringsState.active = true;
	onMount(() => () => (ringsState.active = false));

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
</script>

<svelte:window onkeydown={onescape} />

<svelte:head>
	<link
		rel="stylesheet"
		href="https://fonts.googleapis.com/css2?family=Share+Tech&family=JetBrains+Mono&display=swap"
	/>
</svelte:head>

<div class="mines" transition:fade={{ duration: 300 }}>
	<MatrixBackground />

	<svg class="rings" viewBox="-1 {CENTER_OFFSET - VIEW_HEIGHT} 2 {VIEW_HEIGHT}" preserveAspectRatio="xMidYMax meet">
		{#each rings.toReversed() as ring}
			<circle
				r={RING_WIDTH * (ring + 1)}
				role="button"
				tabindex="0"
				aria-label="Ring {ring}"
				onclick={() => choose(ring)}
				onkeydown={(event) => onkeydown(event, ring)}
			/>
		{/each}
		{#each rings as ring}
			<text y={ring === 0 ? -RING_WIDTH / 3 : -RING_WIDTH * (ring + 0.5)}>Ring {ring}</text>
		{/each}
	</svg>

	<Dialog bind:open={() => open, (value) => value || !open || dismiss()} title="Ring {selected}" />

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
		background: var(--background);
		color: var(--text);
		font-family: 'Share Tech', sans-serif;
		color-scheme: dark;
	}

	.mines :global(:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.rings {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
	}

	circle {
		fill: transparent;
		stroke: var(--ring);
		stroke-width: 1.5;
		cursor: pointer;
	}

	.mines circle:focus-visible {
		outline: none;
		stroke: var(--accent);
		stroke-width: 3;
	}

	text {
		fill: var(--ring-label);
		font-size: 18px;
		text-anchor: middle;
		dominant-baseline: middle;
		pointer-events: none;
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
