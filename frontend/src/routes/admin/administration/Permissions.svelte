<script lang="ts">
	import { params, update } from '#lib/url.svelte.js';
	import { ACTIONS, TABS, type Permissions } from '#lib/admin/access.js';

	type Person = { id: number; name: string | null; email: string | null; hca_id: string };
	type Elevated = { user: Person; permissions: Permissions; created_at: string };
	type List = {
		can_write: boolean;
		superadmins: { hca_id: string; user: Person | null }[];
		users: Elevated[];
	};

	let { onchange }: { onchange: () => void } = $props();

	const COLUMNS = [...TABS, ...ACTIONS];

	let list = $state<List | null>(null);
	let error = $state('');
	let busy = $state(false);
	const search = $derived(params().get('grant') ?? '');
	let candidates = $state<Person[]>([]);

	const display = (person: Person) => person.name ?? person.email ?? `User #${person.id}`;

	async function load() {
		const response = await fetch('/api/admin/permissions', { cache: 'no-cache' });
		if (!response.ok) {
			error = `Couldn't load permissions (${response.status})`;
			return;
		}
		list = await response.json();
	}

	async function write(method: string, id: number, body?: unknown) {
		busy = true;
		const response = await fetch(`/api/admin/permissions/${id}`, {
			method,
			headers: body ? { 'Content-Type': 'application/json' } : undefined,
			body: body ? JSON.stringify(body) : undefined
		}).catch(() => null);
		busy = false;
		error = !response ? 'Network error' : response.ok ? '' : (await response.text()) || `Request failed (${response.status})`;
		await load();
		onchange();
		return !!response?.ok;
	}

	const set = (row: Elevated, changes: Partial<Permissions>) => write('PUT', row.user.id, { permissions: changes });

	const allOn = (row: Elevated) => COLUMNS.every(({ slug }) => row.permissions[slug]);
	const someOn = (row: Elevated) => COLUMNS.some(({ slug }) => row.permissions[slug]);

	function toggleAll(row: Elevated) {
		const value = !allOn(row);
		set(row, Object.fromEntries(COLUMNS.map(({ slug }) => [slug, value])));
	}

	function remove(row: Elevated) {
		if (confirm(`Remove ${display(row.user)} from elevated access?`)) write('DELETE', row.user.id);
	}

	async function add(person: Person) {
		if (await write('POST', person.id)) {
			update({ grant: null }, { replace: true });
			candidates = [];
		}
	}

	let timer: ReturnType<typeof setTimeout>;
	$effect(() => {
		const query = search.trim();
		clearTimeout(timer);
		if (!query) {
			candidates = [];
			return;
		}
		timer = setTimeout(async () => {
			const response = await fetch(`/api/admin/permissions/search?${new URLSearchParams({ q: query })}`, {
				cache: 'no-cache'
			});
			if (response.ok && search.trim() === query) candidates = await response.json();
		}, 200);
		return () => clearTimeout(timer);
	});

	load();
</script>

<section>
	<div class="head">
		<h2>Elevated access</h2>
		{#if list && !list.can_write}
			<span class="note">Read only. Only superadmins from SUPERADMINS can change permissions.</span>
		{/if}
	</div>

	{#if list?.can_write}
		<div class="add">
			<input type="search" placeholder="Add a user: search name, email, Slack, HCA or id"
				value={search}
				oninput={(event) => update({ grant: event.currentTarget.value || null }, { replace: true })}
			/>
			{#if candidates.length}
				<ul>
					{#each candidates as person (person.id)}
						<li>
							<button disabled={busy} onclick={() => add(person)}>
								<span>{display(person)}</span>
								<span class="muted">#{person.id} {person.email ?? ''}</span>
							</button>
						</li>
					{/each}
				</ul>
			{:else if search.trim()}
				<p class="empty">No matching users without access</p>
			{/if}
		</div>
	{/if}

	{#if error}<p class="error">{error}</p>{/if}

	{#if list}
		<div class="scroll">
			<table>
				<thead>
					<tr>
						<th rowspan="2">User</th>
						<th rowspan="2" class="check">All</th>
						<th colspan={TABS.length} class="group">Tabs</th>
						<th colspan={ACTIONS.length} class="group">User actions</th>
						{#if list.can_write}<th rowspan="2"></th>{/if}
					</tr>
					<tr>
						{#each COLUMNS as { slug, name } (slug)}<th class="check">{name}</th>{/each}
					</tr>
				</thead>
				<tbody>
					{#each list.superadmins as superadmin (superadmin.hca_id)}
						<tr>
							<td>
								{#if superadmin.user}
									<a href="/admin/users?user={superadmin.user.id}">{display(superadmin.user)}</a>
								{:else}
									<span class="muted">{superadmin.hca_id}</span>
								{/if}
								<span class="badge">Superadmin (env)</span>
							</td>
							<td class="check"><input type="checkbox" checked disabled /></td>
							{#each COLUMNS as { slug } (slug)}
								<td class="check"><input type="checkbox" checked disabled /></td>
							{/each}
							{#if list.can_write}<td></td>{/if}
						</tr>
					{/each}
					{#key list}
						{#each list.users as row (row.user.id)}
							<tr>
								<td>
									<a href="/admin/users?user={row.user.id}">{display(row.user)}</a>
									<span class="muted">{row.user.email ?? ''}</span>
								</td>
								<td class="check">
									<input
										type="checkbox"
										checked={allOn(row)}
										indeterminate={someOn(row) && !allOn(row)}
										disabled={!list.can_write || busy}
										onchange={() => toggleAll(row)}
									/>
								</td>
								{#each COLUMNS as { slug } (slug)}
									<td class="check">
										<input
											type="checkbox"
											checked={row.permissions[slug]}
											disabled={!list.can_write || busy}
											onchange={(event) => set(row, { [slug]: event.currentTarget.checked })}
										/>
									</td>
								{/each}
								{#if list.can_write}
									<td class="actions"><button disabled={busy} onclick={() => remove(row)}>Remove</button></td>
								{/if}
							</tr>
						{/each}
					{/key}
				</tbody>
			</table>
		</div>
	{/if}
</section>

<style>
	section {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.head {
		display: flex;
		align-items: baseline;
		gap: 12px;
	}

	h2 {
		margin: 0;
		font-size: 15px;
		font-weight: 500;
	}

	.note,
	.muted,
	.empty {
		color: var(--text-muted);
		font-size: 12px;
	}

	.empty {
		margin: 6px 0 0;
	}

	.add {
		position: relative;
		max-width: 480px;
	}

	input[type='search'] {
		box-sizing: border-box;
		width: 100%;
		padding: 8px 12px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	input[type='search']:focus {
		outline: none;
		border-color: var(--accent);
	}

	ul {
		margin: 4px 0 0;
		padding: 4px;
		list-style: none;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--island);
	}

	li button {
		display: flex;
		gap: 8px;
		align-items: baseline;
		width: 100%;
		border: none;
		background: none;
		text-align: left;
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
	}

	tbody tr:last-child td {
		border-bottom: none;
	}

	tbody tr:hover {
		background: var(--hover);
	}

	.group {
		text-align: center;
		border-left: 1px solid var(--island-border);
	}

	.check {
		text-align: center;
	}

	input[type='checkbox'] {
		accent-color: var(--accent);
		cursor: pointer;
	}

	input[type='checkbox']:disabled {
		cursor: default;
	}

	a {
		color: var(--text);
		text-decoration: none;
	}

	a:hover {
		color: var(--accent);
	}

	.badge {
		margin-left: 6px;
		padding: 1px 6px;
		border: 1px solid var(--accent);
		border-radius: 4px;
		color: var(--accent);
		font-size: 11px;
	}

	.actions {
		text-align: right;
	}

	button {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	button:hover:not(:disabled) {
		background: var(--hover);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.error {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
	}
</style>
