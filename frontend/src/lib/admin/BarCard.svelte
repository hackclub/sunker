<script lang="ts">
	import BarList from './BarList.svelte';
	import type { Row } from './analytics.js';

	let {
		title,
		subtitle,
		rows,
		label,
		note,
		onopen
	}: {
		title: string;
		subtitle: string;
		rows: Row[];
		label?: (name: string) => string;
		note?: string;
		onopen: () => void;
	} = $props();
</script>

<button class="card" onclick={onopen}>
	<span class="head">
		<span class="title">{title}</span>
		<span class="muted">{subtitle}</span>
	</span>
	<BarList {rows} {label} />
	{#if note}<span class="muted">{note}</span>{/if}
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
