<script lang="ts">
	import Dialog from '#lib/admin/Dialog.svelte';
	import { params, update } from '#lib/url.svelte.js';
	import type { Force, PlacedTile } from '#lib/map.js';
	import ForceInput from './ForceInput.svelte';
	import JoelMap, { type Trail } from './JoelMap.svelte';
	import Shell from './Shell.svelte';
	import WaveCard from './WaveCard.svelte';
	import { forceText, key, placeName, request, rings, stamp, tone, total, type Hex, type JoelState, type Wave } from './joel.js';

	let {
		joel,
		tiles,
		placed,
		names,
		enemies,
		view = $bindable(),
		refresh
	}: {
		joel: JoelState;
		tiles: PlacedTile[];
		placed: Set<string>;
		names: Map<string, string>;
		enemies: Map<string, Force>;
		view: { x: number; y: number; scale: number } | null;
		refresh: () => Promise<void>;
	} = $props();

	let fogged = $state(false);
	let planned = $state(true);
	let creating = $state(false);
	let name = $state('');
	let notes = $state('');
	let error = $state('');
	let edited = $state<Force | null>(null);
	let reason = $state('');
	let saving = $state(false);

	const selected = $derived(params().get('tile'));
	const tile = $derived(selected ? tiles.find((t) => key(t.row, t.col) === selected) : undefined);
	const held = $derived(joel.territory.find((t) => key(t.row, t.col) === selected));
	const fog = $derived(rings(tiles, placed, enemies));
	const current = $derived(enemies.get(selected ?? '') ?? {});

	const live = (wave: Wave) => wave.status === 'draft' || wave.status === 'scheduled';
	const scheduled = $derived(joel.waves.filter((wave) => wave.status === 'scheduled').sort((a, b) => a.resolve_at!.localeCompare(b.resolve_at!)));
	const drafts = $derived(joel.waves.filter((wave) => wave.status === 'draft'));
	const history = $derived(joel.waves.filter((wave) => !live(wave)).sort((a, b) => (b.resolved_at ?? b.updated_at).localeCompare(a.resolved_at ?? a.updated_at)));

	const trails = $derived.by((): Trail[] => {
		if (!planned) return [];
		return [...scheduled, ...drafts].flatMap((wave, i) => wave.orders.map((order) => ({ kind: order.kind, path: order.path, tone: tone(i), faded: wave.status === 'draft' })));
	});

	const touching = $derived(
		selected
			? joel.waves.filter((wave) => live(wave) && wave.orders.some((order) => order.path.some(([row, col]) => key(row, col) === selected)))
			: []
	);

	$effect(() => {
		void selected;
		edited = null;
		reason = '';
	});

	function pick(row: number, col: number) {
		update({ tile: selected === key(row, col) ? null : key(row, col) }, { replace: true });
	}

	async function create() {
		error = '';
		try {
			const { id } = await request('POST', '/api/admin/joel/waves', { name, notes });
			await refresh();
			creating = false;
			name = '';
			notes = '';
			update({ wave: id, tile: null });
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function save() {
		if (!tile || !edited) return;
		saving = true;
		error = '';
		try {
			await request('PUT', `/api/admin/joel/tiles/${tile.row}/${tile.col}`, { enemies: edited, reason });
			await refresh();
			edited = null;
			reason = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}

	const visibility = (ring: number | undefined) => (ring === undefined ? 'No enemies here' : ring > 1 ? 'Hidden in fog' : ring === 1 ? 'Misty, enemies visible' : 'On the front line, fully visible');
</script>

<Shell>
	{#snippet map()}
		<JoelMap {tiles} {names} {enemies} fog={fogged ? fog : null} {trails} {selected} bind:view onpick={pick} />
	{/snippet}
	{#snippet tools()}
		<button class="button" aria-pressed={fogged} onclick={() => (fogged = !fogged)}>Player fog</button>
		<button class="button" aria-pressed={planned} onclick={() => (planned = !planned)}>Planned waves</button>
	{/snippet}
	{#snippet panel()}
		<header class="row spread">
			<h1>Joel View</h1>
			<button class="button primary" onclick={() => (creating = true)}>New wave</button>
		</header>

		{#if tile}
			<section class="card">
				<div class="row spread">
					<strong>{placeName(names, [tile.row, tile.col] as Hex)}</strong>
					<button class="button" onclick={() => update({ tile: null }, { replace: true })}>Close</button>
				</div>
				<span class="muted">
					Tile {tile.row}, {tile.col} - {tile.tile.base}{tile.tile.variants.length ? `, ${tile.tile.variants.join(', ')}` : ''}
				</span>
				<span class="muted">{visibility(fog.get(selected!))}</span>
				{#if held?.last_occupied_at}<span class="muted">Last occupied {stamp(held.last_occupied_at)}</span>{/if}
				{#if held?.last_freed_at}<span class="muted">Last freed {stamp(held.last_freed_at)}</span>{/if}
				<ForceInput force={edited ?? current} onchange={(force) => (edited = force)} />
				{#if edited}
					<input type="text" placeholder="Reason (for the audit log)" bind:value={reason} />
					<div class="row">
						<button class="button primary" disabled={saving} onclick={save}>Save silently</button>
						<button class="button" onclick={() => (edited = null)}>Discard</button>
						<span class="muted">{forceText(current)} to {forceText(edited)}</span>
					</div>
				{/if}
				{#if touching.length}
					<span class="muted">Planned here:</span>
					<div class="row">
						{#each touching as wave (wave.id)}
							<button class="button" onclick={() => update({ wave: wave.id, tile: null })}>{wave.name}</button>
						{/each}
					</div>
				{/if}
			</section>
		{:else}
			<p class="muted">Click a tile to inspect it or set its enemies silently. Waves are where announced and hidden moves are planned.</p>
		{/if}
		{#if error}<p class="error">{error}</p>{/if}

		<section>
			<h2>Scheduled</h2>
			{#each scheduled as wave (wave.id)}<WaveCard {wave} now={joel.now} latest={joel.latest} />{:else}<p class="muted">Nothing scheduled.</p>{/each}
		</section>
		<section>
			<h2>Drafts</h2>
			{#each drafts as wave (wave.id)}<WaveCard {wave} now={joel.now} latest={joel.latest} />{:else}<p class="muted">No drafts.</p>{/each}
		</section>
		{#if history.length}
			<section>
				<h2>History</h2>
				{#each history as wave (wave.id)}<WaveCard {wave} now={joel.now} latest={joel.latest} />{/each}
			</section>
		{/if}
		<p class="muted">{[...enemies.values()].reduce((sum, force) => sum + total(force), 0)} enemies on {[...enemies.values()].filter((force) => total(force)).length} tiles.</p>
	{/snippet}
</Shell>

{#if creating}
	<Dialog title="New wave" description="A wave is a set of enemy orders that resolve together, with optional announcements." onclose={() => (creating = false)}>
		<form
			class="form"
			onsubmit={(event) => {
				event.preventDefault();
				create();
			}}
		>
			<label>Name<input type="text" bind:value={name} placeholder="e.g. Desert push" maxlength="100" required /></label>
			<label>Notes<textarea rows="3" bind:value={notes} placeholder="Only admins see this"></textarea></label>
			{#if error}<p class="error">{error}</p>{/if}
			<div class="actions">
				<button type="button" class="button" onclick={() => (creating = false)}>Cancel</button>
				<button class="button primary" disabled={!name.trim()}>Create and open</button>
			</div>
		</form>
	</Dialog>
{/if}

<style>
	h1 {
		margin: 0;
		font-size: 16px;
		font-weight: 500;
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	p {
		margin: 0;
	}

	.spread {
		justify-content: space-between;
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.form label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.form input,
	.form textarea {
		width: 100%;
		box-sizing: border-box;
		padding: 6px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}

	.actions .button {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.actions .button.primary {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--background);
	}

	.actions .button:disabled {
		opacity: 0.5;
	}
</style>
