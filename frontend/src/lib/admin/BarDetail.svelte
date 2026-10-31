<script lang="ts">
	import BarList from './BarList.svelte';
	import RangeControls from './RangeControls.svelte';
	import { params } from '#lib/url.svelte.js';
	import { chosenRange, type Analytics, type Range, type Row } from './analytics.js';

	let {
		data,
		rows,
		label,
		note,
		timeless = false
	}: {
		data: Analytics;
		rows: (range: Range) => Row[];
		label?: (name: string) => string;
		note?: string;
		timeless?: boolean;
	} = $props();

	const { preset, range } = $derived(chosenRange(params().get('range'), 'all', data));
</script>

{#if !timeless}
	<div class="controls">
		<RangeControls {data} {preset} {range} fallback="all" />
	</div>
{/if}

<BarList rows={rows(range)} {label} />

{#if note}<p class="note">{note}</p>{/if}

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
