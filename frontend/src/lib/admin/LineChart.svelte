<script lang="ts">
	import { format, type Point, type Unit } from './analytics.js';

	type Props = {
		points: Point[];
		compare?: (number | null)[];
		unit?: Unit;
		height: number;
		axes?: boolean;
		name?: string;
		compareName?: string;
	};

	let {
		points,
		compare,
		unit = 'count',
		height,
		axes = false,
		name = 'This period',
		compareName = 'Previous period'
	}: Props = $props();

	let width = $state(0);
	let active: number | null = $state(null);
	let svg: SVGSVGElement | undefined = $state();
	let tip = $state(0);

	const pad = $derived(
		axes ? { top: 12, right: 16, bottom: 28, left: 48 } : { top: 8, right: 6, bottom: 4, left: 6 }
	);
	const innerWidth = $derived(Math.max(0, width - pad.left - pad.right));
	const innerHeight = $derived(height - pad.top - pad.bottom);
	const values = $derived(points.map((point) => point.value));

	function niceStep(max: number) {
		const raw = max / 4;
		const magnitude = 10 ** Math.floor(Math.log10(raw));
		const step = [1, 2, 2.5, 5, 10].map((m) => m * magnitude).find((s) => s >= raw) ?? raw;
		return unit === 'count' ? Math.max(1, Math.ceil(step)) : step;
	}

	const step = $derived.by(() => {
		if (unit === 'nps') return 0.5;
		const max = Math.max(0, ...values.map((v) => v ?? 0), ...(compare ?? []).map((v) => v ?? 0));
		if (max > 0) return niceStep(max);
		return unit === 'percent' ? 0.25 : 1;
	});
	const bottom = $derived(unit === 'nps' ? -1 : 0);
	const ticks = $derived([0, 1, 2, 3, 4].map((i) => bottom + i * step));
	const top = $derived(ticks[4]);

	const x = (i: number) =>
		pad.left + (points.length > 1 ? (i / (points.length - 1)) * innerWidth : innerWidth / 2);
	const y = (v: number) => pad.top + innerHeight - ((v - bottom) / (top - bottom)) * innerHeight;

	function segments(series: (number | null)[]) {
		const runs: [number, number][][] = [];
		let run: [number, number][] = [];
		series.forEach((v, i) => {
			if (v === null) {
				if (run.length) runs.push(run);
				run = [];
			} else run.push([x(i), y(v)]);
		});
		if (run.length) runs.push(run);
		return runs;
	}

	const line = (runs: [number, number][][]) =>
		runs.map((run) => run.map(([px, py], i) => `${i ? 'L' : 'M'}${px},${py}`).join('')).join('');

	const area = (runs: [number, number][][]) =>
		runs
			.map(
				(run) =>
					`${line([run])}L${run[run.length - 1][0]},${y(0)}L${run[0][0]},${y(0)}Z`
			)
			.join('');

	const main = $derived(segments(values));
	const previous = $derived(compare ? segments(compare) : []);

	const labels = $derived.by(() => {
		const count = Math.min(points.length, Math.max(2, Math.floor(innerWidth / 90)));
		if (points.length <= 1) return points.map((_, i) => i);
		return [...new Set(Array.from({ length: count }, (_, i) => Math.round((i * (points.length - 1)) / (count - 1))))];
	});

	function pick(event: PointerEvent) {
		if (!svg || !points.length) return;
		const px = event.clientX - svg.getBoundingClientRect().left;
		const ratio = innerWidth ? (px - pad.left) / innerWidth : 0;
		active = Math.min(points.length - 1, Math.max(0, Math.round(ratio * (points.length - 1))));
	}

	function down(event: PointerEvent) {
		if (axes && event.pointerType !== 'mouse') pick(event);
	}

	function leave(event: PointerEvent) {
		if (!axes || event.pointerType === 'mouse') active = null;
	}

	function key(event: KeyboardEvent) {
		const last = points.length - 1;
		const current = active ?? last;
		const next =
			event.key === 'ArrowLeft'
				? current - 1
				: event.key === 'ArrowRight'
					? current + 1
					: event.key === 'Home'
						? 0
						: event.key === 'End'
							? last
							: null;
		if (next === null) return;
		event.preventDefault();
		active = Math.min(last, Math.max(0, next));
	}

	const left = $derived.by(() => {
		if (active === null) return 0;
		const px = x(active);
		if (px + 12 + tip <= width) return px + 12;
		if (px - 12 - tip >= 0) return px - 12 - tip;
		return Math.max(0, Math.min(width - tip, px - tip / 2));
	});
</script>

<svelte:window onpointerdown={(event) => svg?.contains(event.target as Node) || (active = null)} />

<span class="chart" bind:clientWidth={width} style:height="{height}px">
	{#if width}
		<svg
			bind:this={svg}
			{width}
			{height}
			role="slider"
			aria-label={name}
			aria-valuemin={0}
			aria-valuemax={points.length - 1}
			aria-valuenow={active ?? points.length - 1}
			aria-valuetext={points.length
				? `${points[active ?? points.length - 1].label}: ${format(values[active ?? points.length - 1], unit)}`
				: 'No data'}
			tabindex={axes ? 0 : undefined}
			aria-hidden={!axes}
			onpointerdown={down}
			onpointermove={pick}
			onpointerleave={leave}
			onpointercancel={() => (active = null)}
			onfocus={() => (active = active ?? points.length - 1)}
			onblur={() => (active = null)}
			onkeydown={key}
		>
			{#if axes}
				{#each ticks as tick (tick)}
					<line class="grid" x1={pad.left} x2={width - pad.right} y1={y(tick)} y2={y(tick)} />
					<text class="tick" x={pad.left - 8} y={y(tick)} text-anchor="end" dominant-baseline="middle">
						{format(tick, unit)}
					</text>
				{/each}
				{#each labels as i (i)}
					<text
						class="tick"
						x={x(i)}
						y={height - 8}
						text-anchor={labels.length > 1 && i === 0 ? 'start' : labels.length > 1 && i === points.length - 1 ? 'end' : 'middle'}
					>
						{points[i].short}
					</text>
				{/each}
			{:else}
				<line class="grid" x1={pad.left} x2={width - pad.right} y1={y(0)} y2={y(0)} />
			{/if}

			{#if compare}
				<path class="previous" d={line(previous)} />
			{/if}
			<path class="area" d={area(main)} />
			<path class="line" d={line(main)} />
			{#each main.filter((run) => run.length === 1) as [[px, py]] (px)}
				<circle class="lone" cx={px} cy={py} r="2.5" />
			{/each}

			{#if active !== null}
				<line class="crosshair" x1={x(active)} x2={x(active)} y1={pad.top} y2={y(0)} />
				{#if compare?.[active] != null}
					<circle class="dot previous-dot" cx={x(active)} cy={y(compare[active] ?? 0)} r="4" />
				{/if}
				{#if values[active] !== null}
					<circle class="dot" cx={x(active)} cy={y(values[active] ?? 0)} r="4" />
				{/if}
			{/if}
		</svg>

		{#if active !== null}
			<span
				class="tooltip"
				bind:clientWidth={tip}
				style:left="{left}px"
				style:top="{pad.top}px"
				style:visibility={tip ? undefined : 'hidden'}
			>
				<span class="date">{points[active].label}</span>
				<span class="row">
					{#if compare}<span class="key"></span>{/if}
					<strong>{values[active] === null ? 'No data' : format(values[active], unit)}</strong>
					{#if compare}<span class="name">{name}</span>{/if}
				</span>
				{#if compare}
					<span class="row">
						<span class="key previous-key"></span>
						<strong>{compare[active] == null ? 'No data' : format(compare[active], unit)}</strong>
						<span class="name">{compareName}</span>
					</span>
				{/if}
			</span>
		{/if}
	{/if}
</span>

<style>
	.chart {
		display: block;
		position: relative;
		width: 100%;
	}

	svg {
		display: block;
		overflow: visible;
		outline: none;
		touch-action: pan-y;
	}

	svg:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 4px;
		border-radius: 4px;
	}

	.grid {
		stroke: var(--island-border);
		stroke-width: 1;
	}

	.tick {
		fill: var(--text-muted);
		font-size: 11px;
		font-variant-numeric: tabular-nums;
	}

	.area {
		fill: var(--accent);
		fill-opacity: 0.1;
	}

	.line,
	.previous {
		fill: none;
		stroke-width: 2;
		stroke-linejoin: round;
		stroke-linecap: round;
	}

	.line {
		stroke: var(--accent);
	}

	.previous {
		stroke: var(--text-muted);
		stroke-opacity: 0.6;
	}

	.lone {
		fill: var(--accent);
	}

	.crosshair {
		stroke: var(--tip-border);
		stroke-width: 1;
	}

	.dot {
		fill: var(--accent);
		stroke: var(--island);
		stroke-width: 2;
	}

	.previous-dot {
		fill: var(--text-muted);
	}

	.tooltip {
		display: flex;
		flex-direction: column;
		gap: 4px;
		position: absolute;
		padding: 8px 10px;
		border: 1px solid var(--tip-border);
		border-radius: 8px;
		background: var(--background);
		box-shadow: 0 4px 16px var(--shadow);
		white-space: nowrap;
		pointer-events: none;
		z-index: 1;
	}

	.date {
		color: var(--text-muted);
		font-size: 12px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	strong {
		color: var(--text);
		font-weight: 500;
		font-size: 14px;
	}

	.name {
		color: var(--text-muted);
		font-size: 12px;
	}

	.key {
		width: 12px;
		height: 2px;
		border-radius: 1px;
		background: var(--accent);
	}

	.previous-key {
		background: var(--text-muted);
	}
</style>
