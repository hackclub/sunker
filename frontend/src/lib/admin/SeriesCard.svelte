<script lang="ts">
	import LineChart from './LineChart.svelte';
	import {
		delta,
		format,
		headline,
		note,
		points,
		presetRange,
		previousRange,
		type Analytics,
		type Metric
	} from './analytics.js';

	let { metric, data, onopen }: { metric: Metric; data: Analytics; onopen: () => void } = $props();

	const range = $derived(presetRange('30d', data));
	const value = $derived(headline(metric, data, range));
	const change = $derived(delta(metric, value, headline(metric, data, previousRange(range))));
</script>

<button class="card" onclick={onopen}>
	<span class="head">
		<span class="title">{metric.title}</span>
		<span class="range">{metric.kind === 'level' ? 'Now' : 'Last 30 days'}</span>
	</span>
	<span class="figure">
		<span class="value">{format(value, metric.unit)}</span>
		{#if change}
			<span class="delta {change.tone}" title="Compared to the 30 days before">{change.text}</span>
		{/if}
	</span>
	<LineChart points={points(metric, data, range, 'day')} unit={metric.unit} height={64} name={metric.title} />
	{#if note(metric, data)}<span class="note">{note(metric, data)}</span>{/if}
</button>

<style>
	.card {
		display: flex;
		flex-direction: column;
		width: 100%;
		gap: 8px;
		padding: 16px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
		transition: border-color 0.15s;
	}

	.card:hover {
		border-color: var(--tip-border);
	}

	.card:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.head {
		display: flex;
		justify-content: space-between;
		gap: 8px;
	}

	.title {
		font-size: 13px;
	}

	.range,
	.note {
		color: var(--text-muted);
		font-size: 12px;
	}

	.figure {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.value {
		font-size: 28px;
		font-weight: 500;
		line-height: 1.1;
	}

	.delta {
		font-size: 12px;
	}

	.good {
		color: var(--positive);
	}

	.bad {
		color: var(--negative);
	}

	.flat {
		color: var(--text-muted);
	}
</style>
