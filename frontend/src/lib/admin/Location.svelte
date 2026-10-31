<script lang="ts">
	import BarList from './BarList.svelte';
	import Globe from './Globe.svelte';
	import { countryName, type Row } from './analytics.js';

	let { rows, height, paused = false }: { rows: Row[]; height: number; paused?: boolean } = $props();
</script>

<span class="location" style:--globe="{height}px">
	<Globe {rows} {height} {paused} />
	<span class="list" style:max-height="{height}px">
		{#if rows.length}
			<BarList {rows} label={countryName} />
		{:else}
			<span class="empty">No users yet</span>
		{/if}
	</span>
</span>

<style>
	.location {
		display: grid;
		grid-template-columns: minmax(0, var(--globe)) minmax(0, 1fr);
		gap: 32px;
		align-items: start;
	}

	.list {
		display: block;
		padding-right: 8px;
		overflow-y: auto;
	}

	.empty {
		color: var(--text-muted);
		font-size: 13px;
	}

	@media (max-width: 720px) {
		.location {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
