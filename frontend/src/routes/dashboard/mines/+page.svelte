<script lang="ts">
	import { onMount } from 'svelte';
	import { CENTER_OFFSET, RING_COUNT, RING_WIDTH, VIEW_HEIGHT, ringsState } from '#lib/rings.svelte.js';

	const rings = Array.from({ length: RING_COUNT }, (_, k) => k);

	onMount(() => {
		ringsState.active = true;
		return () => (ringsState.active = false);
	});
</script>

<svg viewBox="-1 {CENTER_OFFSET - VIEW_HEIGHT} 2 {VIEW_HEIGHT}" preserveAspectRatio="xMidYMax meet">
	{#each rings as ring}
		<circle r={RING_WIDTH * (ring + 1)} />
	{/each}
	{#each rings as ring}
		<text y={ring === 0 ? -RING_WIDTH / 3 : -RING_WIDTH * (ring + 0.5)}>Ring {ring}</text>
	{/each}
</svg>

<style>
	svg {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
	}

	circle {
		fill: none;
		stroke: #2e3032;
		stroke-width: 1.5;
	}

	text {
		fill: #b4b7b9;
		font-size: 18px;
		text-anchor: middle;
		dominant-baseline: middle;
	}
</style>
