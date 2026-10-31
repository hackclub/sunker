<script lang="ts">
	import { format, type Row } from './analytics.js';

	let { rows, label = (name: string) => name }: { rows: Row[]; label?: (name: string) => string } = $props();

	const max = $derived(Math.max(0, ...rows.map((row) => row.value)));
</script>

<span class="bars">
	{#each rows as row}
		<span class="row">
			<span class="name">{label(row.name)}</span>
			<span class="track">
				<span class="bar" style:width="{max ? (row.value / max) * 100 : 0}%"></span>
			</span>
			<span class="value">
				{format(row.value)}
				{#if row.detail}<span class="detail">{row.detail}</span>{/if}
			</span>
		</span>
	{/each}
</span>

<style>
	.bars {
		display: grid;
		grid-template-columns: max-content 1fr max-content;
		row-gap: 2px;
		column-gap: 12px;
		align-items: center;
	}

	.row {
		display: grid;
		grid-template-columns: subgrid;
		grid-column: 1 / -1;
		align-items: center;
		padding: 4px 6px;
		margin: 0 -6px;
		border-radius: 6px;
	}

	.row:hover {
		background: var(--hover);
	}

	.name {
		color: var(--text-muted);
		font-size: 13px;
	}

	.track {
		display: block;
		height: 16px;
	}

	.bar {
		display: block;
		height: 100%;
		min-width: 2px;
		border-radius: 0 4px 4px 0;
		background: var(--accent);
	}

	.value {
		color: var(--text);
		font-size: 13px;
		font-variant-numeric: tabular-nums;
		text-align: right;
	}

	.detail {
		margin-left: 6px;
		color: var(--text-muted);
		font-size: 12px;
	}
</style>
