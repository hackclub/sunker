<script lang="ts" module>
	import { geoCentroid, geoGraticule10, geoOrthographic, geoPath } from 'd3-geo';
	import { feature } from 'topojson-client';
	import type { GeometryCollection, Topology } from 'topojson-specification';
	import world from 'world-atlas/countries-110m.json';
	import { PALETTE, rgb } from '#lib/colors.js';
	import { ALPHA2 } from './countries.js';

	const topology = world as unknown as Topology<{ countries: GeometryCollection<{ name: string }> }>;
	const shapes = feature(topology, topology.objects.countries).features.map((shape) => ({
		code: ALPHA2[String(shape.id)] ?? '',
		name: shape.properties.name,
		shape
	}));
	const graticule = geoGraticule10();

	const mix = (from: string, to: string, t: number) => {
		const [a, b] = [rgb(from), rgb(to)];
		return `#${a.map((c, i) => Math.round((c + (b[i] - c) * t) * 255).toString(16).padStart(2, '0')).join('')}`;
	};
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { countryName, type Row } from './analytics.js';

	let { rows, height, paused = false }: { rows: Row[]; height: number; paused?: boolean } = $props();

	let width = $state(0);
	let hovered: number | null = $state(null);
	let hovering = $state(false);
	let pointer = $state({ x: 0, y: 0 });
	let drag: { x: number; y: number; lambda: number; phi: number; moved: boolean } | null = $state(null);
	let moved = false;

	const start = () => {
		const top = rows.find((row) => row.name !== 'unknown');
		const shape = shapes.find((s) => s.code === top?.name);
		if (!shape) return { lambda: 0, phi: -20 };
		const [lon, lat] = geoCentroid(shape.shape);
		return { lambda: -lon, phi: Math.max(-50, Math.min(50, -lat)) };
	};
	let rotation = $state(start());

	const counts = $derived(new Map(rows.map((row) => [row.name, row.value])));
	const total = $derived(rows.reduce((sum, row) => sum + row.value, 0));
	const max = $derived(Math.max(0, ...rows.filter((row) => row.name !== 'unknown').map((row) => row.value)));
	const radius = $derived(Math.max(0, Math.min(width, height) / 2 - 2));
	const path = $derived(
		geoPath(
			geoOrthographic()
				.scale(radius)
				.translate([width / 2, height / 2])
				.rotate([rotation.lambda, rotation.phi])
		)
	);

	const fill = (code: string) => {
		const count = counts.get(code) ?? 0;
		return count && max ? mix(PALETTE.hover, mix(PALETTE.accent, '#ffffff', 0.45), 0.2 + 0.8 * Math.sqrt(count / max)) : PALETTE.hover;
	};

	onMount(() => {
		if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		let last = performance.now();
		let frame = requestAnimationFrame(function spin(now) {
			if (!paused && !hovering && !drag) rotation.lambda += (now - last) * 0.006;
			last = now;
			frame = requestAnimationFrame(spin);
		});
		return () => cancelAnimationFrame(frame);
	});

	function down(event: PointerEvent) {
		event.currentTarget instanceof Element && event.currentTarget.setPointerCapture(event.pointerId);
		drag = { x: event.clientX, y: event.clientY, ...rotation, moved: false };
	}

	function move(event: PointerEvent) {
		const box = (event.currentTarget as Element).getBoundingClientRect();
		pointer = { x: event.clientX - box.left, y: event.clientY - box.top };
		const index = (event.target as Element).getAttribute('data-index');
		hovered = index === null ? null : Number(index);
		if (!drag) return;
		const [dx, dy] = [event.clientX - drag.x, event.clientY - drag.y];
		if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
		const speed = 60 / Math.max(radius, 1);
		rotation = {
			lambda: drag.lambda + dx * speed,
			phi: Math.max(-70, Math.min(70, drag.phi - dy * speed))
		};
	}

	function up() {
		moved = !!drag?.moved;
		drag = null;
	}

	function cancel() {
		moved = false;
		drag = null;
	}

	function click(event: MouseEvent) {
		if (!moved) return;
		moved = false;
		event.stopPropagation();
		event.preventDefault();
	}

	const active = $derived(hovered === null || drag ? null : shapes[hovered]);
	const activeCount = $derived(active ? (counts.get(active.code) ?? 0) : 0);
</script>

<span class="globe" bind:clientWidth={width} style:height="{height}px">
	{#if width}
		<svg
			{width}
			{height}
			aria-hidden="true"
			class:dragging={drag}
			onpointerenter={() => (hovering = true)}
			onpointerleave={() => {
				hovering = false;
				hovered = null;
			}}
			onpointerdown={down}
			onpointermove={move}
			onpointerup={up}
			onpointercancel={cancel}
			onclickcapture={click}
		>
			<path class="sphere" d={path({ type: 'Sphere' })} />
			<path class="graticule" d={path(graticule)} />
			{#each shapes as shape, i (i)}
				{@const d = path(shape.shape)}
				{#if d}
					<path
						class="country"
						class:active={hovered === i}
						{d}
						fill={fill(shape.code)}
						data-index={i}
					/>
				{/if}
			{/each}
		</svg>
		{#if active}
			<span class="tooltip" class:flip={pointer.x > width / 2} style:left="{pointer.x}px" style:top="{pointer.y}px">
				<span class="name">{active.code ? countryName(active.code) : active.name}</span>
				<span class="row">
					<strong>{activeCount.toLocaleString('en')}</strong>
					<span class="share">
						{total ? `${((activeCount / total) * 100).toFixed(activeCount / total < 0.1 ? 1 : 0)}%` : '0%'}
					</span>
				</span>
			</span>
		{/if}
	{/if}
</span>

<style>
	.globe {
		display: block;
		position: relative;
		width: 100%;
	}

	svg {
		display: block;
		cursor: grab;
		touch-action: none;
	}

	svg.dragging {
		cursor: grabbing;
	}

	.sphere {
		fill: var(--background);
		stroke: var(--island-border);
		stroke-width: 1;
	}

	.graticule {
		fill: none;
		stroke: var(--text);
		stroke-opacity: 0.05;
		stroke-width: 1;
	}

	.country {
		stroke: var(--island);
		stroke-width: 0.5;
	}

	.country.active {
		stroke: var(--text);
		stroke-width: 1;
	}

	.tooltip {
		display: flex;
		flex-direction: column;
		gap: 2px;
		position: absolute;
		transform: translate(12px, 12px);
		padding: 8px 10px;
		border: 1px solid var(--tip-border);
		border-radius: 8px;
		background: var(--background);
		box-shadow: 0 4px 16px var(--shadow);
		white-space: nowrap;
		pointer-events: none;
		z-index: 1;
	}

	.tooltip.flip {
		transform: translate(calc(-100% - 12px), 12px);
	}

	.name,
	.share {
		color: var(--text-muted);
		font-size: 12px;
	}

	.row {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	strong {
		color: var(--text);
		font-size: 14px;
		font-weight: 500;
	}
</style>
