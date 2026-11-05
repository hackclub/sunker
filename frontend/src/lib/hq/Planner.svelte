<script lang="ts">
	import { tick } from 'svelte';
	import { RESOURCE_NAMES } from '#lib/admin/analytics.js';
	import { describe } from '#lib/effects.js';
	import { planner, type Hex } from '#lib/hq/planner.svelte.js';
	import Status from '#lib/hq/Status.svelte';
	import { request } from '#lib/joel/joel.js';
	import { DEFAULT_TILE, type HexMap, type Territory } from '#lib/map.js';
	import { invalidateAll } from '$app/navigation';
	import { page } from '$app/state';

	type Resources = Record<string, number>;
	type Route = { cost: Resources; tiles: number };
	type Choice = { slug: string; name: string; description: string; cost: Resources };
	type Breakdown = { operation: Resources; transport: Resources; cuprite: number; multiplier: number; total: Resources };
	type Operation = {
		id: number;
		name: string;
		status: 'draft' | 'gathering' | 'completed';
		tile: Hex | null;
		route: 'land' | 'sea' | null;
		kind: string | null;
		cuprite: number;
		max_cuprite: number;
		operation: { name: string; description: string } | null;
		allocated: Resources;
		balance: Resources;
		breakdown: Breakdown | null;
		path: Hex[] | null;
		selectable?: Hex[];
		routes?: { land: Route | null; sea: Route | null };
		options?: Choice[];
	};

	const ROUTES = [
		['land', 'Land route'],
		['sea', 'Sea route']
	] as const;

	let { id, onback }: { id: number; onback: () => void } = $props();

	let operation = $state<Operation | null>(null);
	let picking = $state(false);
	let busy = $state(false);
	let error = $state('');
	let amounts = $state<Resources>({});
	let first = $state<HTMLElement>();

	const map = $derived(page.data.map as HexMap);
	const territory = $derived((page.data.territory ?? []) as Territory[]);

	const costs = (resources: Resources) => Object.entries(resources).filter(([, amount]) => amount > 0);
	const named = (resource: string) => RESOURCE_NAMES[resource] ?? resource;
	const percent = (value: number) => `${Math.round(value * 100)}%`;

	const place = $derived.by(() => {
		if (!operation?.tile) return null;
		const [row, col] = operation.tile;
		const tile = map.rows[row]?.[col];
		if (!tile) return null;
		const region = map.regions?.find((region) => region.tiles.some(([r, c]) => r === row && c === col))?.name;
		const held = territory.some((spot) => spot.row === row && spot.col === col && spot.occupied);
		return { title: describe({ ...DEFAULT_TILE, ...tile }).title, region, held };
	});

	const remaining = $derived(
		Object.fromEntries(Object.entries(operation?.breakdown?.total ?? {}).map(([resource, total]) => [resource, Math.max(0, total - (operation?.allocated[resource] ?? 0))]))
	);
	const most = (resource: string) => Math.max(0, Math.min(remaining[resource] ?? 0, operation?.balance[resource] ?? 0));
	const ready = $derived(Object.values(remaining).every((amount) => amount === 0));

	async function call(method: string, url: string, body?: unknown) {
		busy = true;
		try {
			operation = await request(method, url, body);
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		busy = false;
	}

	const save = (changes: Partial<Operation>) => call('PATCH', `/api/operations/${id}`, changes);

	$effect(() => {
		call('GET', `/api/operations/${id}`);
	});

	$effect(() => {
		planner.selectable = picking && operation?.status === 'draft' ? (operation.selectable ?? []) : null;
		planner.selected = operation?.tile ?? null;
		planner.path = operation?.path ?? null;
		planner.pick = (tile) => {
			if (busy) return;
			picking = false;
			save({ tile });
		};
	});

	$effect(() => () => Object.assign(planner, { selectable: null, selected: null, path: null, pick: null }));

	$effect(() => {
		if (picking) tick().then(() => first?.scrollIntoView({ block: 'nearest' }));
	});

	$effect(() => {
		if (operation?.status !== 'gathering') return;
		amounts = Object.fromEntries(Object.keys(remaining).map((resource) => [resource, 0]));
	});

	async function launch() {
		await call('POST', `/api/operations/${id}/launch`);
		if (!error) await invalidateAll();
	}
</script>

<div class="planner">
	<header>
		<button class="back" aria-label="Back to operations" onclick={onback}>
			<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
		</button>
		<h3>{operation?.name ?? ''}</h3>
		{#if operation}<Status status={operation.status} />{/if}
	</header>

	{#snippet where()}
		{#if place}
			<p class="place">
				<strong>{place.title}</strong>
				{#if place.region}<span class="hint">{place.region}</span>{/if}
				{#if place.held}<span class="held">Enemy-held</span>{/if}
			</p>
		{/if}
	{/snippet}

	{#snippet resources(list: Resources)}
		{#each costs(list) as [resource, amount]}
			<span class="resource">{amount} {named(resource)}</span>
		{:else}
			<span class="hint">Free</span>
		{/each}
	{/snippet}

	{#if !operation}
		<p class="hint">{error || 'Loading operation...'}</p>
	{:else if operation.status === 'draft'}
		<ol class="steps">
			<li class="step" bind:this={first}>
				<h4><span class="number">1</span>Pick a tile</h4>
				{@render where()}
				<button class="button" class:primary={!operation.tile && !picking} disabled={busy} onclick={() => (picking = !picking)}>
					{picking ? 'Cancel' : operation.tile ? 'Change tile' : 'Pick a tile'}
				</button>
				{#if picking}<p class="hint">Click one of the highlighted tiles on the map</p>{/if}
			</li>

			<li class="step" class:locked={!operation.tile}>
				<h4><span class="number">2</span>Route</h4>
				{#if operation.routes}
					{#each ROUTES as [kind, label]}
						{@const route = operation.routes[kind]}
						{#if route}
							<button class="choice" class:active={operation.route === kind} disabled={busy} onclick={() => save({ route: kind })}>
								<span class="title">{label}<span class="hint">{route.tiles} {route.tiles === 1 ? 'tile' : 'tiles'}</span></span>
								<span class="costs">{@render resources(route.cost)}</span>
							</button>
						{/if}
					{/each}
					{#if !operation.routes.land && !operation.routes.sea}<p class="hint">No route reaches this tile</p>{/if}
				{/if}
			</li>

			<li class="step" class:locked={!operation.route}>
				<h4><span class="number">3</span>Operation</h4>
				{#if operation.route}
					{#each operation.options ?? [] as choice (choice.slug)}
						<button class="choice" class:active={operation.kind === choice.slug} disabled={busy} onclick={() => save({ kind: choice.slug })}>
							<span class="title">{choice.name}</span>
							<span class="hint">{choice.description}</span>
							<span class="costs">{@render resources(choice.cost)}</span>
						</button>
					{:else}
						<p class="hint">No operations can happen on this tile</p>
					{/each}
				{/if}
			</li>

			<li class="step" class:locked={!operation.breakdown}>
				<h4><span class="number">4</span>Resources</h4>
				{#if operation.breakdown}
					{@const breakdown = operation.breakdown}
					<table>
						<thead>
							<tr><th>Resource</th><th>Operation</th><th>Transport</th><th>Total</th></tr>
						</thead>
						<tbody>
							{#each costs(breakdown.total) as [resource, total]}
								<tr>
									<td>{named(resource)}</td>
									<td>{breakdown.operation[resource] || ''}</td>
									<td>{breakdown.transport[resource] || ''}</td>
									<td>{total}</td>
								</tr>
							{/each}
						</tbody>
					</table>
					<div class="cuprite">
						<div>
							<span class="label">Requested Cuprite</span>
							<span class="hint">Each one cuts all costs by another 25%, now {percent(breakdown.multiplier)} of full price</span>
						</div>
						<div class="stepper">
							<button class="button" aria-label="Less cuprite" disabled={busy || operation.cuprite <= 0} onclick={() => save({ cuprite: operation!.cuprite - 1 })}>-</button>
							<span>{operation.cuprite}</span>
							<button class="button" aria-label="More cuprite" disabled={busy || operation.cuprite >= operation.max_cuprite} onclick={() => save({ cuprite: operation!.cuprite + 1 })}>+</button>
						</div>
					</div>
					<button class="button primary" disabled={busy} onclick={() => call('POST', `/api/operations/${id}/confirm`)}>Confirm</button>
				{/if}
			</li>
		</ol>
	{:else}
		<section class="summary">
			{@render where()}
			{#if operation.operation}
				<p><strong>{operation.operation.name}</strong> <span class="hint">by {operation.route}</span></p>
				<p class="hint">{operation.operation.description}</p>
			{/if}
		</section>
		{#if operation.status === 'gathering'}
			<table class="gather">
				<thead>
					<tr><th>Resource</th><th>Allocated</th><th>You have</th><th></th></tr>
				</thead>
				<tbody>
					{#each costs(operation.breakdown?.total ?? {}) as [resource, total]}
						<tr class:done={!remaining[resource]}>
							<td>{named(resource)}</td>
							<td>{operation.allocated[resource] ?? 0} / {total}</td>
							<td>{operation.balance[resource] ?? 0}</td>
							<td>
								{#if remaining[resource]}
									<div class="stepper">
										<button class="button" aria-label="Less {named(resource)}" disabled={busy || !amounts[resource]} onclick={() => (amounts[resource] -= 1)}>-</button>
										<span>{amounts[resource] ?? 0}</span>
										<button class="button" aria-label="More {named(resource)}" disabled={busy || (amounts[resource] ?? 0) >= most(resource)} onclick={() => (amounts[resource] = (amounts[resource] ?? 0) + 1)}>+</button>
										<button class="button all" disabled={busy || (amounts[resource] ?? 0) >= most(resource)} onclick={() => (amounts[resource] = most(resource))}>All</button>
									</div>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
			<footer>
				<button class="button" disabled={busy} onclick={() => call('POST', `/api/operations/${id}/draft`)}>Back to draft</button>
				<span class="grow"></span>
				{#if !ready}
					<button class="button" disabled={busy || Object.values(amounts).every((amount) => !amount)} onclick={() => call('POST', `/api/operations/${id}/allocate`, amounts)}>Allocate</button>
				{/if}
				<button class="button primary" disabled={busy || !ready} onclick={launch}>Launch</button>
			</footer>
		{:else}
			<p class="hint">This operation has launched.</p>
		{/if}
	{/if}

	{#if operation && error}<p class="error">{error}</p>{/if}
</div>

<style>
	.planner {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	h3 {
		flex: 1;
		margin: 0;
		overflow: hidden;
		font: 18px 'Share Tech', sans-serif;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.back {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}

	.back:hover {
		background: var(--hover);
		color: var(--text);
	}

	.back svg {
		width: 20px;
		height: 20px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.steps {
		display: flex;
		flex-direction: column;
		gap: 12px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.step,
	.summary {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		padding: 12px 16px 16px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	.step.locked {
		opacity: 0.5;
	}

	h4 {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0;
		font: 16px 'Share Tech', sans-serif;
	}

	.number {
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		border-radius: 4px;
		background: var(--background);
		color: var(--text-muted);
		font: 500 12px 'Inter', sans-serif;
	}

	p {
		margin: 0;
	}

	.place {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 8px;
	}

	.held {
		color: var(--negative);
		font-size: 12px;
	}

	.hint {
		color: var(--text-muted);
		font-size: 12px;
	}

	.label {
		display: block;
		font-weight: 500;
	}

	.choice {
		display: flex;
		flex-direction: column;
		gap: 4px;
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-radius: 4px;
		background: var(--background);
		color: var(--text);
		font: 13px 'Inter', sans-serif;
		text-align: left;
		cursor: pointer;
	}

	.choice:hover:not(:disabled) {
		border-color: var(--accent);
	}

	.choice.active {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 15%, var(--background));
	}

	.choice:disabled {
		cursor: default;
	}

	.title {
		display: flex;
		align-items: baseline;
		gap: 8px;
		font-weight: 500;
	}

	.costs {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 12px;
	}

	.resource {
		color: var(--text);
		font-size: 12px;
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th {
		color: var(--text-muted);
		font-size: 12px;
		font-weight: 500;
		text-align: left;
	}

	th,
	td {
		padding: 6px 8px 6px 0;
		border-bottom: 1px solid var(--island-border);
	}

	td:last-child,
	th:last-child {
		padding-right: 0;
		text-align: right;
	}

	tr.done td {
		color: var(--positive);
	}

	td .stepper {
		justify-content: flex-end;
	}

	.cuprite {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		width: 100%;
	}

	.cuprite > div:first-child {
		flex: 1 1 160px;
		min-width: 0;
	}

	.stepper {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.stepper .button {
		width: 32px;
		padding: 4px 0;
	}

	.stepper .button.all {
		width: auto;
		padding: 4px 8px;
	}

	.stepper span {
		min-width: 20px;
		text-align: center;
	}

	footer {
		display: flex;
		gap: 8px;
	}

	.grow {
		flex: 1;
	}

	.error {
		color: var(--negative);
		font-size: 12px;
	}

	@media (orientation: portrait) and (max-width: 600px) {
		.gather,
		.gather thead,
		.gather tbody {
			display: block;
		}

		.gather tr {
			display: grid;
			grid-template-columns: minmax(0, 1fr) auto auto;
			column-gap: 12px;
			padding: 6px 0;
			border-bottom: 1px solid var(--island-border);
		}

		.gather th,
		.gather td {
			padding: 0;
			border: none;
		}

		.gather th:last-child {
			display: none;
		}

		.gather td:last-child {
			grid-column: 1 / -1;
			padding-top: 6px;
		}

		.gather td:last-child:empty {
			display: none;
		}
	}

	@media (pointer: coarse) {
		.back {
			width: 44px;
			height: 44px;
			margin: -8px 0 -8px -12px;
		}

		.stepper .button {
			width: 44px;
			min-height: 44px;
		}
	}
</style>
