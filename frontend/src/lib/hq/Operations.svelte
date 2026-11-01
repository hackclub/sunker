<script lang="ts">
	import Dialog from '#lib/Dialog.svelte';
	import Planner from '#lib/hq/Planner.svelte';
	import Status from '#lib/hq/Status.svelte';
	import { request } from '#lib/joel/joel.js';
	import { params, update } from '#lib/url.svelte.js';
	import { page } from '$app/state';

	type Summary = { id: number; name: string; status: 'draft' | 'gathering' | 'completed' };

	let operations = $state<Summary[] | null>(null);
	let broken = $state(false);
	let naming = $state(false);
	let name = $state('');
	let error = $state('');
	let creating = $state(false);

	const current = $derived(Number(params().get('operation')) || null);

	$effect(() => {
		if (current) return;
		request('GET', '/api/operations')
			.then((list) => {
				operations = list;
				broken = false;
			})
			.catch(() => (broken = true));
	});

	function open(id: number) {
		const mapped = params().has('map');
		update({ operation: id, map: true }, { replace: mapped, state: mapped ? page.state : { map: true } });
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (creating) return;
		creating = true;
		try {
			const { id } = await request('POST', '/api/operations', { name });
			naming = false;
			name = '';
			open(id);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		creating = false;
	}
</script>

{#if current}
	{#key current}
		<Planner id={current} onback={() => update({ operation: null }, { replace: true, state: page.state })} />
	{/key}
{:else}
	<div class="operations">
		<button
			class="button primary"
			onclick={() => {
				error = '';
				naming = true;
			}}
		>
			Create New Operation
		</button>
		{#if broken}
			<p class="muted">Couldn't load your operations</p>
		{:else if !operations}
			<p class="muted">Loading your operations...</p>
		{:else}
			{#each operations as operation (operation.id)}
				<button class="card" onclick={() => open(operation.id)}>
					<span class="name">{operation.name}</span>
					<Status status={operation.status} />
					<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
				</button>
			{:else}
				<p class="muted">No operations yet</p>
			{/each}
		{/if}
	</div>
{/if}

<Dialog bind:open={naming} title="New operation">
	<form onsubmit={create}>
		<label class="field">
			<span class="label">Operation Name</span>
			<input class="input" type="text" maxlength="100" placeholder="Operation name" bind:value={name} />
		</label>
		{#if error}<p class="error">{error}</p>{/if}
		<footer>
			<button type="button" class="button" onclick={() => (naming = false)}>Cancel</button>
			<button class="button primary" disabled={creating || !name.trim()}>Create</button>
		</footer>
	</form>
</Dialog>

<style>
	.operations {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.card {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 12px 12px 16px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: none;
		color: var(--text);
		font: 13px 'Inter', sans-serif;
		text-align: left;
		cursor: pointer;
	}

	.card:hover {
		background: var(--hover);
	}

	.name {
		flex: 1;
		overflow: hidden;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	svg {
		width: 20px;
		height: 20px;
		fill: none;
		stroke: var(--text-muted);
		stroke-width: 2;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.label {
		font-weight: 500;
	}

	.error {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
	}

	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
