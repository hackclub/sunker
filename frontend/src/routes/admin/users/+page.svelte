<script lang="ts">
	import { params, update } from '#lib/url.svelte.js';
	import { Paged } from '#lib/admin/paging.svelte.js';
	import UserPanel from './UserPanel.svelte';
	import { format, type Summary } from './users';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const search = $derived(params().get('q') ?? '');
	let sentinel: HTMLElement;
	let scroller: HTMLElement;

	const selected = $derived(Number(params().get('user')) || null);

	const users = new Paged<Summary>('/api/admin/users', 'users', 'users', () => {
		const query = new URLSearchParams();
		if (search.trim()) query.set('q', search.trim());
		return query;
	});

	$effect(() => {
		search;
		return users.reload();
	});

	$effect(() => {
		users.items.length;
		return users.watch(sentinel);
	});

	const open = (id: number | null) => update({ user: id });

	async function refresh() {
		if (!selected) return;
		const params = new URLSearchParams({ q: String(selected), before: String(selected + 1) });
		const response = await fetch(`/api/admin/users?${params}`, { cache: 'no-cache' });
		if (!response.ok) return;
		const data: { users: Summary[] } = await response.json();
		const fresh = data.users.find((user) => user.id === selected);
		if (fresh) users.items = users.items.map((user) => (user.id === fresh.id ? fresh : user));
	}
</script>

<svelte:head>
	<title>Users - Hack Club Sunker Admin</title>
</svelte:head>

<div class="users" class:split={selected !== null}>
	<div class="list">
		<div class="top">
			<h1>Users</h1>
			<input
				type="search"
				placeholder="Search name, email, Slack, HCA, Hackatime, referral code, handle or id"
				value={search}
				oninput={(event) => {
					update({ q: event.currentTarget.value || null }, { replace: true });
					scroller.scrollTo({ top: 0 });
				}}
			/>
		</div>
		<div class="scroll" bind:this={scroller}>
			<table>
				<thead>
					<tr>
						<th>Id</th>
						<th>Name</th>
						<th>Email</th>
						<th>Handle</th>
						<th>Guild</th>
						<th class="number">Scrip</th>
						<th class="number">Projects</th>
						<th class="number">Ships</th>
						<th>Last seen</th>
						<th>Joined</th>
					</tr>
				</thead>
				<tbody>
					{#each users.items as user (user.id)}
						<tr class:active={user.id === selected} onclick={() => open(user.id)}>
							<td class="muted"><button type="button">{user.id}</button></td>
							<td>
								{user.name ?? ''}
								{#if user.is_banned}<span class="badge">banned</span>{/if}
								{#if user.deleted_at}<span class="badge">deleted</span>{/if}
							</td>
							<td>{user.email ?? ''}</td>
							<td>{user.handle ?? ''}</td>
							<td>{user.guild ?? ''}</td>
							<td class="number">{user.scrip ?? 0}</td>
							<td class="number">{user.projects}</td>
							<td class="number">{user.ships}</td>
							<td class="muted">{format(user.last_seen)}</td>
							<td class="muted">{new Date(user.created_at).toLocaleDateString()}</td>
						</tr>
					{/each}
				</tbody>
			</table>
			<div bind:this={sentinel} class="status">
				{#if users.error}
					<span class="error">{users.error}</span>
				{:else if users.loading}
					Loading...
				{:else if users.items.length === 0}
					No users found
				{:else if !users.more}
					{users.items.length} users
				{/if}
			</div>
		</div>
	</div>
	{#if selected !== null}
		<UserPanel id={selected} permissions={data.admin.permissions} onopen={open} onclose={() => open(null)} onchange={refresh} />
	{/if}
</div>

<style>
	.users {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 16px;
		height: 100%;
		color-scheme: dark;
	}

	.users.split {
		grid-template-columns: minmax(0, 1fr) minmax(0, 1.3fr);
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 12px;
		min-height: 0;
	}

	.top {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	h1 {
		margin: 0;
		font-size: 20px;
		font-weight: 500;
	}

	input {
		padding: 8px 12px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.scroll {
		flex: 1;
		min-height: 0;
		overflow: auto;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 13px;
	}

	th {
		position: sticky;
		top: 0;
		background: var(--island);
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

	tbody tr {
		cursor: pointer;
	}

	tbody tr:hover {
		background: var(--hover);
	}

	tr.active {
		background: var(--hover);
		box-shadow: inset 2px 0 var(--accent);
	}

	td button {
		padding: 0;
		border: none;
		background: none;
		color: inherit;
		font: inherit;
		cursor: inherit;
	}

	.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	.muted {
		color: var(--text-muted);
	}

	.badge {
		margin-left: 6px;
		padding: 1px 6px;
		border: 1px solid var(--negative);
		border-radius: 4px;
		color: var(--negative);
		font-size: 11px;
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

	@media (max-width: 900px), (max-height: 480px) {
		.users {
			position: relative;
		}

		.users.split {
			grid-template-columns: minmax(0, 1fr);
		}
	}

	@media (pointer: coarse) {
		input {
			padding: 12px;
		}

		td {
			padding: 14px 10px;
		}
	}
</style>
