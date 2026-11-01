<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/admin/Dialog.svelte';
	import Segmented from '#lib/admin/Segmented.svelte';
	import { RESOURCE_NAMES } from '#lib/admin/analytics.js';
	import { request } from '#lib/joel/joel.js';
	import type { StockedPrize, Contract } from '#lib/mines.js';
	import { RING_COUNT } from '#lib/rings.svelte.js';

	type Kind = 'contracts' | 'prizes';
	type Draft = {
		key: string | number | null;
		ring: number;
		name: string;
		description: string;
		image_url: string;
		video_url: string;
		criteria: string;
		resources: Record<string, number>;
		tagline: string;
		base: number;
		additional: number;
		links: string;
	};

	const RINGS = Array.from({ length: RING_COUNT }, (_, i) => RING_COUNT - 1 - i);
	const KINDS: { value: Kind; label: string }[] = [
		{ value: 'contracts', label: 'Contracts' },
		{ value: 'prizes', label: 'Prizes' }
	];

	let mines = $state<{ contracts: Contract[]; prizes: StockedPrize[] } | null>(null);
	let kind = $state<Kind>('contracts');
	let draft = $state<Draft | null>(null);
	let error = $state('');
	let failure = $state('');
	let saving = $state(false);
	let deleting = $state(false);

	const dollars = (cents: number) => `$${(cents / 100).toFixed(2)}`;

	async function refresh() {
		try {
			mines = await request('GET', '/api/admin/mines');
			failure = '';
		} catch (e) {
			failure = `Couldn't load the mines: ${e instanceof Error ? e.message : e}`;
		}
	}

	onMount(refresh);

	function open(item?: Contract | StockedPrize) {
		error = '';
		deleting = false;
		const contract = item && 'slug' in item ? item : undefined;
		const prize = item && 'id' in item ? item : undefined;
		draft = {
			key: contract?.slug ?? prize?.id ?? null,
			ring: item?.ring ?? RING_COUNT - 1,
			name: item?.name ?? '',
			description: item?.description ?? '',
			image_url: contract?.image_url ?? '',
			video_url: contract?.video_url ?? '',
			criteria: contract?.criteria.join('\n') ?? '',
			resources: { ...contract?.resources },
			tagline: prize?.tagline ?? '',
			base: (prize?.base_cost_cents ?? 0) / 100,
			additional: (prize?.additional_cost_cents ?? 0) / 100,
			links: prize?.fulfillment_links.join('\n') ?? ''
		};
	}

	const split = (text: string) => text.split('\n').map((line) => line.trim()).filter(Boolean);

	async function save() {
		if (!draft) return;
		saving = true;
		error = '';
		const body =
			kind === 'contracts'
				? {
						ring: draft.ring,
						name: draft.name,
						description: draft.description,
						image_url: draft.image_url,
						video_url: draft.video_url,
						criteria: split(draft.criteria),
						resources: draft.resources
					}
				: {
						ring: draft.ring,
						name: draft.name,
						tagline: draft.tagline,
						description: draft.description,
						base_cost_cents: Math.round(draft.base * 100),
						additional_cost_cents: Math.round(draft.additional * 100),
						fulfillment_links: split(draft.links)
					};
		const url = `/api/admin/mines/${kind}`;
		try {
			await (draft.key === null ? request('POST', url, body) : request('PUT', `${url}/${draft.key}`, body));
			await refresh();
			draft = null;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		saving = false;
	}

	async function remove() {
		if (!draft || draft.key === null) return;
		if (!deleting) {
			deleting = true;
			return;
		}
		saving = true;
		try {
			await request('DELETE', `/api/admin/mines/${kind}/${draft.key}`);
			await refresh();
			draft = null;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		saving = false;
	}
</script>

<svelte:head>
	<title>Mines - Hack Club Sunker Admin</title>
</svelte:head>

<div class="mines">
	<header>
		<h1>Mines</h1>
		<Segmented options={KINDS} value={kind} label="Show" onchange={(value) => (kind = value)} />
		<button class="button primary" onclick={() => open()}>{kind === 'contracts' ? 'New contract' : 'New prize'}</button>
	</header>

	{#if !mines}
		<p class="muted">{failure || 'Loading the mines...'}</p>
	{:else}
		{#each RINGS as ring (ring)}
			{@const items = kind === 'contracts' ? mines.contracts.filter((t) => t.ring === ring) : mines.prizes.filter((p) => p.ring === ring)}
			<section>
				<h2>Ring {ring}</h2>
				{#if !items.length}
					<p class="muted">Nothing in this ring yet</p>
				{/if}
				<div class="grid">
					{#each items as item ('slug' in item ? item.slug : item.id)}
						<button class="item" onclick={() => open(item)}>
							{#if 'slug' in item}
								<strong>{item.name}</strong>
								<span class="muted">{item.criteria.length} criteria{item.video_url ? ', video' : ''}{item.image_url ? ', image' : ''}</span>
								{#if Object.keys(item.resources).length}
									<span class="resources">{Object.entries(item.resources).map(([name, amount]) => `+${amount} ${RESOURCE_NAMES[name] ?? name}`).join(', ')}</span>
								{/if}
							{:else}
								{#if item.tagline}<em class="muted">{item.tagline}</em>{/if}
								<strong>{item.name}</strong>
								<span class="muted">{dollars(item.base_cost_cents)} base, {dollars(item.additional_cost_cents)} additional, {item.fulfillment_links.length} links</span>
							{/if}
						</button>
					{/each}
				</div>
			</section>
		{/each}
	{/if}
</div>

{#if draft}
	<Dialog title={draft.key === null ? (kind === 'contracts' ? 'New contract' : 'New prize') : draft.name || 'Untitled'} onclose={() => (draft = null)}>
		<form
			onsubmit={(event) => {
				event.preventDefault();
				save();
			}}
		>
			<div class="pair">
				<label>Name<input type="text" bind:value={draft.name} maxlength="100" required /></label>
				<label>
					Ring
					<select bind:value={draft.ring}>
						{#each RINGS as ring (ring)}
							<option value={ring}>Ring {ring}</option>
						{/each}
					</select>
				</label>
			</div>
			{#if kind === 'prizes'}
				<label>Tagline<input type="text" bind:value={draft.tagline} placeholder="Shown in italics above the name" /></label>
			{/if}
			<label>Description<textarea rows="4" bind:value={draft.description}></textarea></label>
			{#if kind === 'contracts'}
				<div class="pair">
					<label>Image URL<input type="url" bind:value={draft.image_url} placeholder="https://" /></label>
					<label>Video URL<input type="url" bind:value={draft.video_url} placeholder="https://" /></label>
				</div>
				<label>Criteria<textarea rows="4" bind:value={draft.criteria} placeholder="One per line"></textarea></label>
				<fieldset>
					<legend>Resources gained on completion</legend>
					<div class="amounts">
						{#each Object.entries(RESOURCE_NAMES) as [key, name] (key)}
							<label>{name}<input type="number" min="0" step="1" value={draft.resources[key] ?? 0} oninput={(event) => (draft!.resources[key] = Number(event.currentTarget.value))} /></label>
						{/each}
					</div>
				</fieldset>
			{:else}
				<fieldset>
					<legend>Internal</legend>
					<div class="pair">
						<label>Base cost ($)<input type="number" min="0" step="0.01" bind:value={draft.base} /></label>
						<label>Additional cost ($)<input type="number" min="0" step="0.01" bind:value={draft.additional} /></label>
					</div>
					<label>Fulfillment links<textarea rows="3" bind:value={draft.links} placeholder="One per line"></textarea></label>
				</fieldset>
			{/if}
			{#if error}<p class="error">{error}</p>{/if}
			<footer>
				{#if draft.key !== null}
					<button type="button" class="button danger" class:primary={deleting} disabled={saving} onclick={remove}>{deleting ? 'Confirm delete' : 'Delete'}</button>
				{/if}
				<span class="grow"></span>
				<button type="button" class="button" onclick={() => (draft = null)}>Cancel</button>
				<button type="submit" class="button primary" disabled={saving}>{saving ? 'Saving...' : 'Save'}</button>
			</footer>
		</form>
	</Dialog>
{/if}

<style>
	.mines {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	header {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	h1 {
		margin: 0 auto 0 0;
		font-size: 20px;
		font-weight: 500;
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	h2 {
		margin: 0;
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 8px;
	}

	.item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 4px;
		padding: 12px 16px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
		text-align: left;
		cursor: pointer;
	}

	.item:hover {
		background: var(--hover);
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
		font-size: 12px;
	}

	.resources {
		color: var(--positive);
		font-size: 12px;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}

	fieldset {
		display: flex;
		flex-direction: column;
		gap: 12px;
		margin: 0;
		padding: 12px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
	}

	legend {
		padding: 0 4px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.amounts {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
		gap: 8px;
	}

	input,
	select,
	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: 6px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	textarea {
		resize: vertical;
	}

	footer {
		display: flex;
		gap: 8px;
	}

	.grow {
		flex: 1;
	}

	.error {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
	}

	.button {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.button:hover:not(:disabled) {
		background: var(--hover);
	}

	.button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.button.primary {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--background);
	}

	.button.danger {
		color: var(--negative);
	}

	.button.primary.danger {
		border-color: var(--negative);
		background: var(--negative);
		color: var(--background);
	}
</style>
