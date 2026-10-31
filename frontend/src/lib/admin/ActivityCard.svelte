<script lang="ts">
	import ActivityMap from './ActivityMap.svelte';
	import { ACTIVITY, activity, type Analytics, type Range } from './analytics.js';

	let { kind, data, range, onopen }: { kind: keyof typeof ACTIVITY; data: Analytics; range: Range; onopen: () => void } =
		$props();

	const config = $derived(ACTIVITY[kind]);
</script>

<button class="card" onclick={onopen}>
	<span class="head">
		<span class="title">{config.title}</span>
		<span class="muted">All time</span>
	</span>
	<ActivityMap grid={activity(data.activity[kind], range)} unit={config.unit} noun={config.noun} />
	{#if config.note && !Object.keys(data.activity[kind]).length}<span class="muted">{config.note}</span>{/if}
</button>

<style>
	.card {
		display: flex;
		flex-direction: column;
		width: 100%;
		gap: 12px;
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

	.muted {
		color: var(--text-muted);
		font-size: 12px;
	}
</style>
