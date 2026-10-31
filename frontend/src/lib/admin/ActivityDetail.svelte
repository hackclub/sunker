<script lang="ts">
	import ActivityMap from './ActivityMap.svelte';
	import RangeControls from './RangeControls.svelte';
	import { params } from '#lib/url.svelte.js';
	import { ACTIVITY, activity, chosenRange, type Analytics } from './analytics.js';

	let { kind, data }: { kind: keyof typeof ACTIVITY; data: Analytics } = $props();

	const { preset, range } = $derived(chosenRange(params().get('range'), 'all', data));
	const config = $derived(ACTIVITY[kind]);
</script>

<div class="controls">
	<RangeControls {data} {preset} {range} fallback="all" />
</div>

<ActivityMap grid={activity(data.activity[kind], range)} unit={config.unit} noun={config.noun} />

{#if config.note && !Object.keys(data.activity[kind]).length}<p class="note">{config.note}</p>{/if}

<style>
	.controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px 12px;
	}

	.note {
		margin: 0;
		color: var(--text-muted);
		font-size: 12px;
	}
</style>
