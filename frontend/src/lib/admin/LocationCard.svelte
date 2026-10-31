<script lang="ts">
	import Location from './Location.svelte';
	import type { Row } from './analytics.js';

	let { rows, onopen, paused = false }: { rows: Row[]; onopen: () => void; paused?: boolean } = $props();
</script>

<button class="card" onclick={onopen}>
	<span class="head">
		<span class="title">Where people are from</span>
		<span class="muted">All time</span>
	</span>
	<Location {rows} height={320} {paused} />
</button>

<style>
	.card {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 100%;
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
