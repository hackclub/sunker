<script lang="ts">
	import { onMount } from 'svelte';
	import type { OrderStatus } from '#lib/orders.js';

	type Order = { id: number; status: OrderStatus; created_at: string; prize: string };

	let orders = $state<Order[] | null>(null);
	let broken = $state(false);

	onMount(() => {
		fetch('/api/orders', { cache: 'no-cache' })
			.then((response) => (response.ok ? response.json() : Promise.reject()))
			.then((data) => (orders = data.orders))
			.catch(() => (broken = true));
	});
</script>

{#if broken}
	<p class="muted">Couldn't load your orders</p>
{:else if !orders}
	<p class="muted">Loading your orders...</p>
{:else}
	<ul>
		{#each orders as order (order.id)}
			<li>
				<span class="what">
					<strong>{order.prize}</strong>
					<span class="muted">{new Date(order.created_at).toLocaleDateString()}</span>
				</span>
				<span class="status {order.status}">{order.status}</span>
			</li>
		{:else}
			<p class="muted">No orders yet</p>
		{/each}
	</ul>
{/if}

<style>
	ul {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
	}

	.what {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	strong {
		overflow: hidden;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
	}

	.status {
		padding: 3px 8px;
		border: 1px solid currentColor;
		border-radius: 999px;
		color: var(--text-muted);
		font-size: 11px;
		text-transform: capitalize;
	}

	.rejected {
		color: var(--negative);
	}

	.ordered {
		color: var(--accent);
	}

	.fulfilled {
		color: var(--positive);
	}
</style>
