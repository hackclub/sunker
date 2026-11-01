<script lang="ts">
	import { untrack } from 'svelte';
	import Dialog from '#lib/admin/Dialog.svelte';
	import { update } from '#lib/url.svelte.js';
	import type { Force, PlacedTile } from '#lib/map.js';
	import AnnouncementEditor from './AnnouncementEditor.svelte';
	import ForceInput from './ForceInput.svelte';
	import JoelMap, { type Trail } from './JoelMap.svelte';
	import Shell from './Shell.svelte';
	import {
		KINDS,
		STATUSES,
		TONES,
		adjacent,
		followup,
		forceText,
		fromInput,
		key,
		localInput,
		placeName,
		relative,
		request,
		rings,
		route,
		simulate,
		stamp,
		total,
		warning,
		type Hex,
		type JoelState,
		type Order,
		type OrderKind,
		type Wave
	} from './joel.js';

	let {
		wave,
		joel,
		tiles,
		placed,
		names,
		enemies,
		view = $bindable(),
		refresh
	}: {
		wave: Wave;
		joel: JoelState;
		tiles: PlacedTile[];
		placed: Set<string>;
		names: Map<string, string>;
		enemies: Map<string, Force>;
		view: { x: number; y: number; scale: number } | null;
		refresh: () => Promise<void>;
	} = $props();

	type Pending = { title: string; description: string; label: string; danger?: boolean; run: () => Promise<void> };

	const clone = (list: Order[]) => list.map((order) => ({ ...order, force: { ...order.force }, path: order.path.map(([row, col]) => [row, col] as Hex) }));

	const start = untrack(() => wave);
	let orders = $state(clone(start.orders));
	let name = $state(start.name);
	let notes = $state(start.notes);
	let synced = start.updated_at;
	let dirty = $state(false);
	let saving = $state(false);
	let error = $state('');
	let tool = $state<OrderKind | null>(null);
	let drawing = $state<number | null>(null);
	let focus = $state<number | null>(null);
	let after = $state(false);
	let fogged = $state(false);
	let resolveAt = $state(localInput(start.resolve_at ?? new Date(Date.now() + 3600000).toISOString()));
	let pending = $state<Pending | null>(null);
	let timer: ReturnType<typeof setTimeout> | undefined;

	const editable = $derived(wave.status === 'draft');
	const finished = $derived(wave.status === 'resolved' || wave.status === 'rolled_back');
	const offset = $derived(new Date(joel.now).getTime() - Date.now());
	const projection = $derived(simulate(enemies, orders));
	const available = $derived(orders.map((order, i) => simulate(enemies, orders.slice(0, i)).after.get(key(...order.path[0])) ?? {}));
	const shown = $derived(after && !finished ? projection.after : enemies);
	const fog = $derived(fogged ? rings(tiles, placed, shown) : null);
	const problems = $derived(
		orders.flatMap((order, i) => [
			...(total(order.force) ? [] : [`Order ${i + 1} has no enemies`]),
			...(order.kind === 'move' && order.path.length < 2 ? [`Order ${i + 1} needs a destination`] : [])
		])
	);

	const trails = $derived.by((): Trail[] => {
		const planned = orders.map((order, i) => ({ kind: order.kind, path: order.path, tone: TONES[i % TONES.length], bold: focus === i, faded: finished && focus !== i }));
		if (!finished) return planned.filter((_, i) => i !== drawing);
		const walked = orders.flatMap((order, i) =>
			order.kind === 'move' && order.steps?.length ? [{ kind: order.kind, path: order.steps.map((step) => [step.row, step.col] as Hex), tone: TONES[i % TONES.length], bold: focus === i }] : []
		);
		return [...planned, ...walked];
	});
	const draft = $derived(drawing !== null && orders[drawing] ? { path: orders[drawing].path, tone: TONES[drawing % TONES.length] } : null);

	$effect(() => {
		if (wave.updated_at === synced || dirty || drawing !== null) return;
		synced = wave.updated_at;
		orders = clone(wave.orders);
		name = wave.name;
		notes = wave.notes;
	});

	function changed() {
		dirty = true;
		clearTimeout(timer);
		timer = setTimeout(flush, 700);
	}

	async function flush() {
		clearTimeout(timer);
		if (!dirty || saving) return;
		if (editable && (problems.length || drawing !== null)) return;
		saving = true;
		try {
			await request('PUT', `/api/admin/joel/waves/${wave.id}`, {
				name,
				notes,
				orders: editable ? orders.map(({ kind, force, path }) => ({ kind, force, path })) : null
			});
			dirty = false;
			error = '';
			await refresh();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}

	function pick(row: number, col: number) {
		const hex: Hex = [row, col];
		if (drawing !== null) {
			const path = orders[drawing].path;
			const last = path.at(-1)!;
			const index = path.findIndex(([r, c]) => r === row && c === col);
			if (index === path.length - 1) drawing = null;
			else if (index >= 0) path.splice(index + 1);
			else path.push(...(adjacent(last, hex) ? [hex] : route(last, hex, placed)));
			changed();
			return;
		}
		if (!tool || !editable) return;
		const here = projection.after.get(key(row, col)) ?? {};
		const order: Order = { kind: tool, path: [hex], force: tool === 'spawn' || !total(here) ? { chunker: 1 } : { ...here } };
		orders.push(order);
		focus = orders.length - 1;
		if (tool === 'move') drawing = orders.length - 1;
		tool = null;
		changed();
	}

	function shift(i: number, by: number) {
		const [order] = orders.splice(i, 1);
		orders.splice(i + by, 0, order);
		focus = i + by;
		changed();
	}

	function drop(i: number) {
		orders.splice(i, 1);
		focus = null;
		if (drawing === i) drawing = null;
		changed();
	}

	async function act(action: string, body: object = {}) {
		await flush();
		if (dirty) throw new Error('Fix the orders before doing that');
		await request('POST', `/api/admin/joel/waves/${wave.id}/${action}`, body);
		await refresh();
	}

	function confirm(next: Pending) {
		pending = next;
	}

	async function run() {
		if (!pending) return;
		const { run } = pending;
		pending = null;
		try {
			await run();
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function attempt(action: string, body: object = {}) {
		try {
			await act(action, body);
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	function key_(event: KeyboardEvent) {
		if ((event.target as HTMLElement).closest('input, textarea')) return;
		if (event.key === 'Escape') {
			drawing = null;
			tool = null;
		}
		if (event.key === 'Backspace' && drawing !== null && orders[drawing].path.length > 1) {
			orders[drawing].path.pop();
			changed();
		}
	}

	const describe = (order: Order) => {
		const from = placeName(names, order.path[0]);
		if (order.kind !== 'move') return from;
		return order.path.length > 1 ? `${from} to ${placeName(names, order.path.at(-1)!)} (${order.path.length - 1} steps)` : `${from} to ...`;
	};

	const projected = $derived(orders.map((order, i) => ({ ...order, taken: projection.results[i]?.taken, delivered: order.kind === 'move' ? projection.results[i]?.taken : undefined })));
</script>

<svelte:window onkeydown={key_} onbeforeunload={(event) => dirty && event.preventDefault()} />

<Shell>
	{#snippet map()}
		<JoelMap {tiles} {names} enemies={shown} {fog} {trails} {draft} bind:view onpick={pick} />
	{/snippet}
	{#snippet tools()}
		{#if !finished}
			<button class="button" aria-pressed={!after} onclick={() => (after = false)}>Before</button>
			<button class="button" aria-pressed={after} onclick={() => (after = true)}>After this wave</button>
		{/if}
		<button class="button" aria-pressed={fogged} onclick={() => (fogged = !fogged)}>Player fog</button>
		{#if drawing !== null}
			<span class="hint">Click tiles to extend the path, a far tile to route there, a path tile to cut back to it, the end tile or Escape to finish. Backspace removes a step.</span>
		{:else if tool}
			<span class="hint">Click the tile where this {KINDS[tool].toLowerCase()} {tool === 'move' ? 'starts' : 'happens'}. Escape cancels.</span>
		{/if}
	{/snippet}
	{#snippet panel()}
		<header>
			<button class="back" onclick={() => update({ wave: null })}>All waves</button>
			<div class="title">
				<input class="name" type="text" bind:value={name} oninput={changed} onblur={flush} maxlength="100" aria-label="Wave name" />
				<span class="badge {wave.status}">{STATUSES[wave.status]}</span>
			</div>
			<span class="muted">
				Created {stamp(wave.created_at)}{wave.creator ? ` by ${wave.creator}` : ''} -
				{saving ? 'Saving...' : dirty ? (problems.length || drawing !== null ? 'Not saved yet' : 'Unsaved') : 'Saved'}
			</span>
			<textarea rows="2" bind:value={notes} oninput={changed} onblur={flush} placeholder="Private notes"></textarea>
		</header>
		{#if error}<p class="error">{error}</p>{/if}
		{#if editable && problems.length}<p class="error">{problems.join('\n')}</p>{/if}

		<section>
			<h2>Orders</h2>
			{#if editable}
				<div class="row">
					{#each ['move', 'spawn', 'retreat'] as const as kind (kind)}
						<button class="button" aria-pressed={tool === kind} disabled={drawing !== null} onclick={() => (tool = tool === kind ? null : kind)}>Add {KINDS[kind].toLowerCase()}</button>
					{/each}
				</div>
			{/if}
			{#each orders as order, i (i)}
				<div class="card order" class:focused={focus === i} style:--tone={TONES[i % TONES.length]}>
					<button class="summary" onclick={() => (focus = focus === i ? null : i)}>
						<span class="dot"></span>
						<strong>{i + 1}. {KINDS[order.kind]}</strong>
						<span class="muted">{describe(order)}</span>
					</button>
					<ForceInput force={order.force} disabled={!editable} available={order.kind === 'spawn' || finished ? undefined : available[i]} onchange={(force) => ((order.force = force), changed())} />
					{#if !finished && order.kind !== 'spawn' && projection.results[i]?.short}
						<span class="error">Only {forceText(projection.results[i].taken)} will be there by then.</span>
					{/if}
					{#if finished}
						<span class="muted">
							{#if order.kind === 'move'}
								Took {forceText(order.taken ?? {})}, {total(order.delivered ?? {}) ? `${forceText(order.delivered ?? {})} arrived` : 'none arrived'}{order.steps?.some((step) => step.halted) ? ', halted' : ''}.
							{:else if order.kind === 'retreat'}
								Removed {forceText(order.taken ?? {})}.
							{:else}
								Spawned {forceText(order.taken ?? {})}.
							{/if}
						</span>
						{#if order.kind === 'move' && order.steps?.some((step) => total(step.lost))}
							<ul>
								{#each order.steps.filter((step) => total(step.lost)) as step (`${step.row},${step.col}`)}
									<li>{placeName(names, [step.row, step.col])}: lost {forceText(step.lost)}{step.halted ? ', halted' : ''}</li>
								{/each}
							</ul>
						{/if}
					{/if}
					{#if editable}
						<div class="row">
							{#if order.kind === 'move'}
								<button class="button" aria-pressed={drawing === i} onclick={() => (drawing = drawing === i ? null : i)}>{drawing === i ? 'Done' : 'Edit path'}</button>
								<button class="button" onclick={() => ((order.path = [order.path[0]]), (drawing = i), changed())}>Redraw</button>
							{/if}
							<button class="button" disabled={i === 0} onclick={() => shift(i, -1)} aria-label="Move up">Up</button>
							<button class="button" disabled={i === orders.length - 1} onclick={() => shift(i, 1)} aria-label="Move down">Down</button>
							<button class="button danger" onclick={() => drop(i)}>Remove</button>
						</div>
					{/if}
				</div>
			{:else}
				<p class="muted">{editable ? 'No orders yet. Add a move, spawn or retreat, then click the map.' : 'This wave has no orders.'}</p>
			{/each}
		</section>

		<section>
			<h2>Timing</h2>
			{#if wave.status === 'draft'}
				<label class="muted">Resolve at<input type="datetime-local" bind:value={resolveAt} /></label>
				<div class="row">
					<button class="button primary" disabled={!orders.length || !resolveAt} onclick={() => attempt('schedule', { resolve_at: fromInput(resolveAt) })}>Schedule</button>
					<button class="button" disabled={!orders.length} onclick={() => confirm({ title: 'Resolve now?', description: 'Every order runs immediately, one tile at a time. Announcements set to go out on resolve will publish.', label: 'Resolve now', run: () => act('resolve') })}>Resolve now</button>
					<button class="button danger" onclick={() => confirm({ title: 'Cancel this wave?', description: 'It stays in the history and can be reopened as a draft.', label: 'Cancel wave', danger: true, run: () => act('cancel') })}>Cancel wave</button>
					<button class="button danger" onclick={() => confirm({ title: 'Delete this wave?', description: 'The wave, its orders and its announcements are deleted for good.', label: 'Delete', danger: true, run: async () => { await request('DELETE', `/api/admin/joel/waves/${wave.id}`); update({ wave: null }); await refresh(); } })}>Delete</button>
				</div>
			{:else if wave.status === 'scheduled'}
				<p>Resolves {stamp(wave.resolve_at!)} ({relative(wave.resolve_at!, Date.now() + offset)}).</p>
				<div class="row">
					<button class="button" onclick={() => attempt('unschedule')}>Back to draft</button>
					<button class="button" onclick={() => confirm({ title: 'Resolve now?', description: 'Every order runs immediately instead of at the scheduled time.', label: 'Resolve now', run: () => act('resolve') })}>Resolve now</button>
					<button class="button danger" onclick={() => confirm({ title: 'Cancel this wave?', description: 'It stays in the history and can be reopened as a draft.', label: 'Cancel wave', danger: true, run: () => act('cancel') })}>Cancel wave</button>
				</div>
			{:else if wave.status === 'resolved'}
				<p>Resolved {stamp(wave.resolved_at!)}.</p>
				{#if joel.latest === wave.id}
					<div class="row">
						<button class="button danger" onclick={() => confirm({ title: 'Roll back this wave?', description: 'Every tile it touched goes back to how it was before it resolved, and its follow-up announcement is unpublished. Silent edits made to those tiles since then are lost.', label: 'Roll back', danger: true, run: () => act('rollback') })}>Roll back</button>
					</div>
				{:else}
					<p class="muted">Only the most recently resolved wave can be rolled back.</p>
				{/if}
			{:else if wave.status === 'rolled_back'}
				<p>Resolved {stamp(wave.resolved_at!)}, then rolled back.</p>
			{:else}
				<div class="row">
					<button class="button" onclick={() => attempt('reopen')}>Reopen as draft</button>
					<button class="button danger" onclick={() => confirm({ title: 'Delete this wave?', description: 'The wave, its orders and its announcements are deleted for good.', label: 'Delete', danger: true, run: async () => { await request('DELETE', `/api/admin/joel/waves/${wave.id}`); update({ wave: null }); await refresh(); } })}>Delete</button>
				</div>
			{/if}
		</section>

		<section>
			<h2>Announcements</h2>
			<p class="muted">Leave both empty to keep this wave hidden from players.</p>
			<AnnouncementEditor {wave} slot="warning" now={joel.now} suggest={() => warning(orders, names)} {refresh} />
			<AnnouncementEditor {wave} slot="followup" now={joel.now} suggest={() => followup(finished ? wave.orders : projected, names)} {refresh} />
		</section>
	{/snippet}
</Shell>

{#if pending}
	<Dialog title={pending.title} description={pending.description} onclose={() => (pending = null)}>
		<div class="confirm">
			<button class="plain" onclick={() => (pending = null)}>Back</button>
			<button class="plain primary" class:danger={pending.danger} onclick={run}>{pending.label}</button>
		</div>
	</Dialog>
{/if}

<style>
	header {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.back {
		align-self: flex-start;
		padding: 0;
		border: none;
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.back::before {
		content: '< ';
	}

	.back:hover {
		color: var(--accent);
	}

	.title {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.title .name {
		flex: 1;
		padding: 4px 6px;
		border-color: transparent;
		background: none;
		font-size: 16px;
		font-weight: 500;
	}

	.title .name:hover,
	.title .name:focus {
		border-color: var(--island-border);
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	p {
		margin: 0;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.hint {
		padding: 5px 10px;
		border: 1px solid var(--accent);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font-size: 12px;
	}

	.order {
		border-left: 3px solid var(--tone);
	}

	.order.focused {
		border-color: var(--tone);
	}

	.summary {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 6px;
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--tone);
	}

	ul {
		margin: 0;
		padding-left: 18px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.confirm {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}

	.plain {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.plain.primary {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--background);
	}

	.plain.primary.danger {
		border-color: var(--negative);
		background: var(--negative);
	}
</style>
