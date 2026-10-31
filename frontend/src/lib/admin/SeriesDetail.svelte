<script lang="ts">
	import BarList from './BarList.svelte';
	import LineChart from './LineChart.svelte';
	import RangeControls from './RangeControls.svelte';
	import Segmented from './Segmented.svelte';
	import { params, update } from '#lib/url.svelte.js';
	import {
		chosenRange,
		delta,
		format,
		headline,
		hourly,
		note,
		points,
		previousRange,
		rangeLabel,
		shares,
		versus,
		type Analytics,
		type Granularity,
		type Metric
	} from './analytics.js';

	let { metric, data }: { metric: Metric; data: Analytics } = $props();

	const { preset, range } = $derived(chosenRange(params().get('range'), '30d', data));
	const granularity: Exclude<Granularity, 'total'> = $derived(
		hourly(range) ? 'hour' : params().get('by') === 'week' ? 'week' : 'day'
	);
	const per = $derived({ hour: 'Hour', day: 'Day', week: 'Week' }[granularity]);
	const cumulative = $derived(params().has('cumulative'));
	const mode = $derived(cumulative ? 'cumulative' : 'period');
	const compare = $derived(params().get('compare') !== 'off');
	const previous = $derived(previousRange(range));
	const current = $derived(points(metric, data, range, granularity, cumulative));
	const before = $derived(points(metric, data, previous, granularity, cumulative));
	const value = $derived(headline(metric, data, range, cumulative));
	const change = $derived(delta(metric, value, headline(metric, data, previous, cumulative)));
	const parts = $derived(
		metric.parts &&
			shares(
				metric.parts.map((part) => ({
					name: part.name,
					value: headline({ ...metric, add: [part.key], sub: undefined, base: undefined }, data, range) ?? 0
				}))
			)
	);
</script>

<div class="summary">
	<span class="value">{format(value, metric.unit)}</span>
	{#if change}
		<span class="delta {change.tone}">{change.text}</span>
		<span class="versus">{versus(range)}</span>
	{/if}
</div>

<div class="controls">
	<RangeControls {data} {preset} {range} fallback="30d" />
	{#if granularity !== 'hour'}
		<Segmented
			label="Granularity"
			options={[
				{ value: 'day', label: 'Day' },
				{ value: 'week', label: 'Week' }
			]}
			value={granularity}
			onchange={(value) => update({ by: value === 'week' ? value : null }, { replace: true })}
		/>
	{/if}
	{#if metric.kind !== 'level'}
		<Segmented
			label="Mode"
			options={[
				{ value: 'period', label: `Per ${per.toLowerCase()}` },
				{ value: 'cumulative', label: 'Cumulative' }
			]}
			value={mode}
		onchange={(value) => update({ cumulative: value === 'cumulative' }, { replace: true })}
		/>
	{/if}
	<label class="toggle">
		<input
			type="checkbox"
			checked={compare}
			onchange={(event) => update({ compare: event.currentTarget.checked ? null : 'off' }, { replace: true })}
		/>
		Compare to previous period
	</label>
</div>

{#if compare}
	<div class="legend">
		<span><span class="key"></span>{rangeLabel(range, data.today)}</span>
		<span><span class="key previous"></span>Previous period</span>
	</div>
{/if}

<LineChart
	points={current}
	compare={compare ? before.map((point) => point.value) : undefined}
	unit={metric.unit}
	height={320}
	axes
	name={metric.title}
/>

{#if parts}
	<BarList rows={parts} />
{/if}

{#if note(metric, data)}<p class="note">{note(metric, data)}</p>{/if}

<details>
	<summary>Data table</summary>
	<table>
		<thead>
			<tr>
				<th>{per}</th>
				<th>{metric.title}</th>
				{#if compare}<th>Previous period</th>{/if}
			</tr>
		</thead>
		<tbody>
			{#each current as point, i (point.label)}
				<tr>
					<td>{point.label}</td>
					<td>{format(point.value, metric.unit)}</td>
					{#if compare}<td>{format(before[i]?.value ?? null, metric.unit)}</td>{/if}
				</tr>
			{/each}
		</tbody>
	</table>
</details>

<style>
	.summary {
		display: flex;
		align-items: baseline;
		gap: 10px;
		margin-top: -8px;
	}

	.value {
		font-size: 40px;
		font-weight: 500;
		line-height: 1;
	}

	.delta,
	.versus {
		font-size: 13px;
	}

	.versus {
		color: var(--text-muted);
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

	.controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px 12px;
	}

	.toggle {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--text-muted);
		font-size: 13px;
		cursor: pointer;
	}

	.toggle input {
		accent-color: var(--accent);
	}

	.legend {
		display: flex;
		gap: 16px;
		margin-bottom: -8px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.legend > span {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.key {
		width: 12px;
		height: 2px;
		border-radius: 1px;
		background: var(--accent);
	}

	.key.previous {
		background: var(--text-muted);
	}

	.note {
		margin: 0;
		color: var(--text-muted);
		font-size: 12px;
	}

	details {
		color: var(--text-muted);
		font-size: 13px;
	}

	summary {
		cursor: pointer;
	}

	table {
		width: 100%;
		margin-top: 8px;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}

	th,
	td {
		padding: 6px 8px;
		border-bottom: 1px solid var(--island-border);
		text-align: right;
	}

	th:first-child,
	td:first-child {
		text-align: left;
	}

	th {
		font-weight: 500;
	}

	td {
		color: var(--text);
	}
</style>
