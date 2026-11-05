<script lang="ts">
	import Segmented from '#lib/admin/Segmented.svelte';
	import { request } from '#lib/joel/joel.js';
	import { ORDER_STATUSES, type OrderStatus } from '#lib/orders.js';

	type Order = {
		id: number;
		user_id: number;
		user_name: string | null;
		status: OrderStatus;
		tracking_number: string | null;
		created_at: string;
		prize: string;
		prize_ring: number;
	};

	const FILTERS = [{ value: 'all', label: 'All' }, ...ORDER_STATUSES.map((status) => ({ value: status, label: status[0].toUpperCase() + status.slice(1) }))];

	let filter = $state<string>('pending');
	let orders = $state<Order[] | null>(null);
	let users = $state(false);
	let failure = $state('');
	let edits = $state<Record<number, { status: OrderStatus; tracking: string }>>({});
	let saving = $state<number | null>(null);
	let errors = $state<Record<number, string>>({});

	async function load() {
		orders = null;
		failure = '';
		try {
			const data = await request('GET', `/api/admin/orders${filter === 'all' ? '' : `?status=${filter}`}`);
			orders = data.orders;
			users = data.users;
			edits = Object.fromEntries(data.orders.map((order: Order) => [order.id, { status: order.status, tracking: order.tracking_number ?? '' }]));
		} catch (e) {
			failure = e instanceof Error ? e.message : String(e);
		}
	}

	$effect(() => {
		filter;
		load();
	});

	const changed = (order: Order) => edits[order.id] && (edits[order.id].status !== order.status || edits[order.id].tracking !== (order.tracking_number ?? ''));

	async function save(order: Order) {
		saving = order.id;
		errors[order.id] = '';
		try {
			await request('POST', `/api/admin/orders/${order.id}`, { status: edits[order.id].status, tracking_number: edits[order.id].tracking });
			order.status = edits[order.id].status;
			order.tracking_number = edits[order.id].tracking.trim() || order.tracking_number;
		} catch (e) {
			errors[order.id] = e instanceof Error ? e.message : String(e);
		}
		saving = null;
	}
</script>

<svelte:head>
	<title>Orders - Hack Club Sunker Admin</title>
</svelte:head>

<div class="orders">
	<header>
		<h1>Orders</h1>
		<Segmented options={FILTERS} value={filter} label="Status" onchange={(value) => (filter = value)} />
	</header>

	{#if !orders}
		<p class="muted">{failure || 'Loading orders...'}</p>
	{:else if !orders.length}
		<p class="muted">No orders here</p>
	{:else}
		<div class="scroll">
			<table>
				<thead>
					<tr><th>#</th><th>Prize</th><th>User</th><th>Ordered</th><th>Status</th><th>Tracking number</th><th></th></tr>
				</thead>
				<tbody>
					{#each orders as order (order.id)}
						<tr>
							<td data-label="#">{order.id}</td>
							<td data-label="Prize">
								<span>{order.prize} <span class="muted">ring {order.prize_ring}</span></span>
							</td>
							<td data-label="User">
								{#if users}
									<a href="/admin/users?user={order.user_id}">{order.user_name ?? `User #${order.user_id}`}</a>
								{:else}
									{order.user_name ?? `User #${order.user_id}`}
								{/if}
							</td>
							<td data-label="Ordered">{new Date(order.created_at).toLocaleDateString()}</td>
							<td data-label="Status">
								<select bind:value={edits[order.id].status} aria-label="Status of order #{order.id}">
									{#each ORDER_STATUSES as status (status)}<option value={status}>{status}</option>{/each}
								</select>
							</td>
							<td data-label="Tracking number"><input type="text" bind:value={edits[order.id].tracking} aria-label="Tracking number of order #{order.id}" /></td>
							<td>
								<button class="button primary" disabled={!changed(order) || saving === order.id} onclick={() => save(order)}>{saving === order.id ? 'Saving...' : 'Save'}</button>
								{#if errors[order.id]}<p class="error">{errors[order.id]}</p>{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<style>
	.orders {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}

	h1 {
		margin: 0 auto 0 0;
		font-size: 20px;
		font-weight: 500;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
		font-size: 12px;
	}

	.scroll {
		overflow-x: auto;
		border: 1px solid var(--island-border);
		border-radius: 8px;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 12px;
	}

	th,
	td {
		padding: 6px 8px;
		border-bottom: 1px solid var(--island-border);
		text-align: left;
		vertical-align: middle;
		white-space: nowrap;
	}

	th {
		color: var(--text-muted);
		font-weight: 500;
	}

	a {
		color: var(--accent);
	}

	input,
	select {
		box-sizing: border-box;
		padding: 4px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
	}

	.error {
		margin: 4px 0 0;
		color: var(--negative);
	}

	.button {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		cursor: pointer;
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

	@media (max-width: 740px) {
		.scroll {
			overflow: visible;
			border: none;
		}

		table,
		tbody {
			display: block;
		}

		thead {
			display: none;
		}

		tr {
			display: grid;
			grid-template-columns: repeat(2, minmax(0, 1fr));
			gap: 10px 12px;
			margin-bottom: 8px;
			padding: 12px;
			border: 1px solid var(--island-border);
			border-radius: 12px;
			background: var(--island);
		}

		td {
			display: flex;
			flex-direction: column;
			gap: 2px;
			grid-column: 1 / -1;
			padding: 0;
			border: none;
			white-space: normal;
		}

		td::before {
			content: attr(data-label);
			color: var(--text-muted);
			font-size: 11px;
		}

		td:nth-child(1) {
			grid-column: 1;
			grid-row: 1;
		}

		td:nth-child(4) {
			grid-column: 2;
			grid-row: 1;
		}

		td:nth-child(7)::before {
			display: none;
		}
	}

	@media (pointer: coarse) {
		input,
		select,
		.button {
			min-height: 44px;
		}
	}

	@media (max-width: 740px) and (pointer: coarse) {
		td a {
			margin: -12px 0;
			padding: 15px 0;
		}
	}
</style>
