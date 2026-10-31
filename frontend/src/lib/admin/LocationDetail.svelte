<script lang="ts">
	import Location from './Location.svelte';
	import RangeControls from './RangeControls.svelte';
	import { params } from '#lib/url.svelte.js';
	import { chosenRange, countries, type Analytics } from './analytics.js';

	let { data }: { data: Analytics } = $props();

	const { preset, range } = $derived(chosenRange(params().get('range'), 'all', data));
</script>

<div class="controls">
	<RangeControls {data} {preset} {range} fallback="all" />
</div>

<Location rows={countries(data, range)} height={480} />

<style>
	.controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px 12px;
	}
</style>
