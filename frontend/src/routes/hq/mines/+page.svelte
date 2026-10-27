<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/Dialog.svelte';
	import { CENTER_OFFSET, RING_COUNT, RING_WIDTH, VIEW_HEIGHT, ringsState } from '#lib/rings.svelte.js';

	const rings = Array.from({ length: RING_COUNT }, (_, k) => k);

	let selected = $state(0);
	let open = $state(false);

	onMount(() => {
		ringsState.active = true;
		return () => (ringsState.active = false);
	});
</script>

<svg viewBox="-1 {CENTER_OFFSET - VIEW_HEIGHT} 2 {VIEW_HEIGHT}" preserveAspectRatio="xMidYMax meet">
	{#each rings.toReversed() as ring}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<circle
			r={RING_WIDTH * (ring + 1)}
			onclick={() => {
				selected = ring;
				open = true;
			}}
		/>
	{/each}
	{#each rings as ring}
		<text y={ring === 0 ? -RING_WIDTH / 3 : -RING_WIDTH * (ring + 0.5)}>Ring {ring}</text>
	{/each}
</svg>

<Dialog bind:open title="Ring {selected}" />

<style>
	svg {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
	}

	circle {
		fill: transparent;
		stroke: #2e3032;
		stroke-width: 1.5;
		cursor: pointer;
	}

	text {
		fill: #b4b7b9;
		font-size: 18px;
		text-anchor: middle;
		dominant-baseline: middle;
		pointer-events: none;
	}
</style>
