<script lang="ts">
	import { WEEKDAYS, format, type Unit } from './analytics.js';

	let { grid, unit = 'count', noun }: { grid: number[][]; unit?: Unit; noun: string } = $props();

	const max = $derived(Math.max(0, ...grid.flat()));
	const sum = $derived(grid.flat().reduce((a, b) => a + b, 0));
	let active: [number, number] | null = $state(null);
	let table: HTMLElement | undefined = $state();

	const amount = (value: number) => `${format(value, unit)} ${noun}`;
	const hour = (h: number) => `${String(h).padStart(2, '0')}:00`;

	function scrub(event: PointerEvent) {
		if (event.pointerType === 'mouse') return;
		const cell = document.elementFromPoint(event.clientX, event.clientY);
		if (cell instanceof HTMLElement && cell.dataset.day !== undefined) active = [Number(cell.dataset.day), Number(cell.dataset.hour)];
	}
</script>

<svelte:window onpointerdown={(event) => table?.contains(event.target as Node) || (active = null)} />

<span class="activity">
	<span class="readout">
		{#if active}
			{WEEKDAYS[active[0]]} {hour(active[1])} UTC: {amount(grid[active[0]][active[1]])}
		{:else}
			{amount(sum)} in total
		{/if}
	</span>
	<span
		class="grid"
		role="img"
		aria-label="{amount(sum)} by weekday and hour"
		bind:this={table}
		onpointerdown={scrub}
		onpointermove={scrub}
		onpointercancel={() => (active = null)}
	>
		{#each grid as row, day (day)}
			<span class="day">{WEEKDAYS[day]}</span>
			{#each row as value, h (h)}
				<span
					class="cell"
					role="presentation"
					style:--shade="{max ? Math.sqrt(value / max) * 100 : 0}%"
					data-day={day}
					data-hour={h}
					onpointerenter={() => (active = [day, h])}
					onpointerleave={(event) => event.pointerType === 'mouse' && (active = null)}
				></span>
			{/each}
		{/each}
		<span></span>
		{#each { length: 24 }, h (h)}
			<span class="hour">{h % 6 ? '' : h}</span>
		{/each}
	</span>
</span>

<style>
	.activity {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.readout {
		color: var(--text-muted);
		font-size: 12px;
		font-variant-numeric: tabular-nums;
	}

	.grid {
		display: grid;
		grid-template-columns: max-content repeat(24, minmax(0, 1fr));
		gap: 3px;
		align-items: center;
		touch-action: pan-y;
	}

	.day,
	.hour {
		color: var(--text-muted);
		font-size: 11px;
	}

	.day {
		padding-right: 6px;
	}

	.cell {
		aspect-ratio: 1;
		border-radius: 3px;
		background: color-mix(in srgb, var(--accent) var(--shade), var(--hover));
	}
</style>
