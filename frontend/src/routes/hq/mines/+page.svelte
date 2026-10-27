<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/Dialog.svelte';
	import MatrixBackground from '#lib/MatrixBackground.svelte';
	import { CENTER_OFFSET, RING_COUNT, RING_WIDTH, VIEW_HEIGHT, ringsState } from '#lib/rings.svelte.js';

	const rings = Array.from({ length: RING_COUNT }, (_, k) => k);

	let selected = $state(0);
	let open = $state(false);

	onMount(() => {
		ringsState.active = true;
		return () => (ringsState.active = false);
	});
</script>

<svelte:head>
	<link
		rel="stylesheet"
		href="https://fonts.googleapis.com/css2?family=Share+Tech&family=JetBrains+Mono:ital,wght@0,100..800;1,100..800&display=swap"
	/>
</svelte:head>

<div class="mines">
	<MatrixBackground />

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
</div>

<style>
	.mines {
		--bg: #0e0f10;
		--island: #1e1f22;
		--island-border: #2b2d30;
		--hover: #2b2d30;
		--text: #dfe1e5;
		--text-muted: #868a91;
		--accent: #8bcf86;
		--radius: 12px;
		--radius-small: 6px;
		position: fixed;
		inset: 0;
		background: var(--bg);
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
