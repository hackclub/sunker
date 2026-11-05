<script lang="ts">
	import { Paged } from '#lib/admin/paging.svelte.js';
	import { params, update } from '#lib/url.svelte.js';

	type Person = { id: number; name: string | null };
	type Action = {
		id: number;
		action: string;
		summary: string;
		reason: string | null;
		created_at: string;
		admin: Person;
		user: Person | null;
	};

	let { version }: { version: number } = $props();

	const search = $derived(params().get('q') ?? '');
	const action = $derived(params().get('action') ?? '');
	const actor = $derived(params().get('admin') ?? '');
	const target = $derived(params().get('user') ?? '');
	const from = $derived(params().get('from') ?? '');
	const to = $derived(params().get('to') ?? '');
	let filters = $state<{ actions: string[]; admins: Person[] }>({ actions: [], admins: [] });
	let sentinel: HTMLElement;

	const display = (person: Person) => person.name ?? `User #${person.id}`;
	const startOf = (date: string, days = 0) => {
		const day = new Date(`${date}T00:00`);
		day.setDate(day.getDate() + days);
		return isNaN(day.getTime()) ? '' : day.toISOString();
	};

	async function loadFilters() {
		const response = await fetch('/api/admin/audit/filters', { cache: 'no-cache' });
		if (response.ok) filters = await response.json();
	}

	const actions = new Paged<Action>('/api/admin/audit', 'actions', 'the audit log', () => {
		const query = new URLSearchParams();
		if (search.trim()) query.set('q', search.trim());
		if (action) query.set('action', action);
		if (actor) query.set('admin', actor);
		if (target.trim()) query.set('user', target.trim());
		if (from) query.set('from', startOf(from));
		if (to) query.set('to', startOf(to, 1));
		return query;
	});

	const filter = (key: string) => (event: Event & { currentTarget: HTMLInputElement | HTMLSelectElement }) =>
		update({ [key]: event.currentTarget.value || null }, { replace: true });

	function clear() {
		update({ q: null, action: null, admin: null, user: null, from: null, to: null }, { replace: true });
	}

	$effect(() => {
		version;
		loadFilters();
	});

	$effect(() => {
		[search, action, actor, target, from, to, version];
		return actions.reload();
	});

	$effect(() => {
		actions.items.length;
		return actions.watch(sentinel);
	});
</script>

<section>
	<h2>Audit log</h2>
	<div class="filters">
		<input class="grow" type="search" placeholder="Search summary, reason, action or names" value={search} oninput={filter('q')} />
		<select value={action} onchange={filter('action')}>
			<option value="">All actions</option>
			{#each filters.actions as name (name)}<option value={name}>{name}</option>{/each}
		</select>
		<select value={actor} onchange={filter('admin')}>
			<option value="">All admins</option>
			{#each filters.admins as admin (admin.id)}<option value={String(admin.id)}>{display(admin)}</option>{/each}
		</select>
		<input type="search" placeholder="Target user" value={target} oninput={filter('user')} />
		<label>From <input type="date" value={from} max={to || undefined} onchange={filter('from')} /></label>
		<label>To <input type="date" value={to} min={from || undefined} onchange={filter('to')} /></label>
		<button onclick={clear}>Clear</button>
	</div>
	<div class="scroll">
		<table>
			<thead>
				<tr>
					<th>Time</th>
					<th>Admin</th>
					<th>Action</th>
					<th>Target</th>
					<th>Summary</th>
					<th>Reason</th>
				</tr>
			</thead>
			<tbody>
				{#each actions.items as entry (entry.id)}
					<tr>
						<td class="muted">{new Date(entry.created_at).toLocaleString()}</td>
						<td><a href="/admin/users?user={entry.admin.id}">{display(entry.admin)}</a></td>
						<td><code>{entry.action}</code></td>
						<td>
							{#if entry.user}
								<a href="/admin/users?user={entry.user.id}">{display(entry.user)}</a>
							{:else}
								<span class="muted">none</span>
							{/if}
						</td>
						<td class="wrap">{entry.summary}</td>
						<td class="wrap muted">{entry.reason ?? ''}</td>
					</tr>
				{/each}
			</tbody>
		</table>
		<div bind:this={sentinel} class="status">
			{#if actions.error}
				<span class="error">{actions.error}</span>
			{:else if actions.loading}
				Loading...
			{:else if actions.items.length === 0}
				No admin actions found
			{:else if !actions.more}
				{actions.items.length} actions
			{/if}
		</div>
	</div>
</section>

<style>
	section {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	h2 {
		margin: 0;
		font-size: 15px;
		font-weight: 500;
	}

	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		align-items: center;
	}

	.grow {
		flex: 1;
		min-width: 240px;
	}

	input,
	select {
		padding: 7px 10px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
		color-scheme: dark;
	}

	input:focus,
	select:focus {
		outline: none;
		border-color: var(--accent);
	}

	label {
		display: flex;
		gap: 6px;
		align-items: center;
		color: var(--text-muted);
		font-size: 12px;
	}

	button {
		padding: 7px 12px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 13px;
		cursor: pointer;
	}

	button:hover {
		background: var(--hover);
	}

	.scroll {
		overflow-x: auto;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		color-scheme: dark;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 13px;
	}

	th {
		color: var(--text-muted);
		font-weight: 500;
		text-align: left;
	}

	th,
	td {
		padding: 8px 10px;
		border-bottom: 1px solid var(--island-border);
		white-space: nowrap;
		vertical-align: top;
	}

	td.wrap {
		min-width: 240px;
		white-space: normal;
	}

	tbody tr:hover {
		background: var(--hover);
	}

	code {
		padding: 1px 6px;
		border-radius: 4px;
		background: var(--hover);
		font-family: 'JetBrains Mono', monospace;
		font-size: 12px;
	}

	a {
		color: var(--text);
		text-decoration: none;
	}

	a:hover {
		color: var(--accent);
	}

	.muted {
		color: var(--text-muted);
	}

	.status {
		padding: 12px;
		color: var(--text-muted);
		font-size: 12px;
		text-align: center;
	}

	.error {
		color: var(--negative);
	}

	select {
		max-width: 100%;
	}

	@media (pointer: coarse) {
		input,
		select,
		button {
			min-height: 44px;
		}

		a {
			padding: 15px 0;
		}
	}
</style>
