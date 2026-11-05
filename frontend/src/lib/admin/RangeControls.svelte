<script lang="ts">
	import Segmented from './Segmented.svelte';
	import { update } from '#lib/url.svelte.js';
	import type { Analytics, Preset, Range } from './analytics.js';

	let { data, preset, range, fallback }: { data: Analytics; preset: Preset; range: Range; fallback: Preset } = $props();

	const options: { value: Preset; label: string }[] = [
		{ value: '24h', label: '24h' },
		{ value: '7d', label: '7d' },
		{ value: '30d', label: '30d' },
		{ value: '90d', label: '90d' },
		{ value: 'all', label: 'All' },
		{ value: 'custom', label: 'Custom' }
	];

	const write = (value: string | null) => update({ range: value }, { replace: true });

	function choose(value: Preset) {
		if (value === 'custom') write(`${range.start.slice(0, 10)}..${range.end.slice(0, 10)}`);
		else write(value === fallback ? null : value);
	}

	function edit(side: 'start' | 'end', value: string) {
		if (!value) return;
		const next = { ...range, [side]: value };
		write(next.start <= next.end ? `${next.start}..${next.end}` : `${next.end}..${next.start}`);
	}
</script>

<Segmented label="Date range" {options} value={preset} onchange={choose} />
{#if preset === 'custom'}
	<span class="custom">
		<input
			type="date"
			aria-label="Start date"
			max={data.today}
			value={range.start}
			onchange={(event) => edit('start', event.currentTarget.value)}
		/>
		<span>to</span>
		<input
			type="date"
			aria-label="End date"
			max={data.today}
			value={range.end}
			onchange={(event) => edit('end', event.currentTarget.value)}
		/>
	</span>
{/if}

<style>
	.custom {
		display: inline-flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		color: var(--text-muted);
		font-size: 13px;
	}

	input {
		padding: 4px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 13px;
		color-scheme: dark;
	}

	input:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -1px;
	}

	@media (pointer: coarse) {
		input {
			min-height: 40px;
		}
	}
</style>
