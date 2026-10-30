<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/Dialog.svelte';
	import MatrixBackground from '#lib/MatrixBackground.svelte';
	import { CENTER_OFFSET, RING_COUNT, RING_WIDTH, VIEW_HEIGHT, ringsState } from '#lib/rings.svelte.js';

	const rings = Array.from({ length: RING_COUNT }, (_, k) => k);

	let selected = $state(0);
	let open = $state(false);

	ringsState.active = true;
	onMount(() => () => (ringsState.active = false));

	function choose(ring: number) {
		selected = ring;
		open = true;
	}

	function onkeydown(event: KeyboardEvent, ring: number) {
		if (event.key !== 'Enter' && event.key !== ' ') return;
		event.preventDefault();
		choose(ring);
	}
</script>

<svelte:head>
	<title>Mines - Hack Club Sunker</title>
	<link
		rel="stylesheet"
		href="https://fonts.googleapis.com/css2?family=Share+Tech&family=JetBrains+Mono&display=swap"
	/>
</svelte:head>

<div class="mines">
	<MatrixBackground />

	<svg viewBox="-1 {CENTER_OFFSET - VIEW_HEIGHT} 2 {VIEW_HEIGHT}" preserveAspectRatio="xMidYMax meet">
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

	<Dialog bind:open title="Ring {selected}" />
</div>

<style>
	.mines {
		--radius: 12px;
		--radius-small: 6px;
		position: fixed;
		inset: 0;
		background: var(--background);
		color: var(--text);
		font-family: 'Share Tech', sans-serif;
		color-scheme: dark;
	}

	.mines :global(:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	svg {
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
</style>
