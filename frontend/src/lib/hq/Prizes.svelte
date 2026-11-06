<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/Dialog.svelte';
	import AllPrizes from '#lib/hq/AllPrizes.svelte';
	import PrizeCard from '#lib/hq/PrizeCard.svelte';
	import { browse, catalog, type Prize } from '#lib/contracts.svelte.js';
	import { request } from '#lib/joel/joel.js';
	import type { OrderStatus } from '#lib/orders.js';
	import { link, load, projects, ring } from '#lib/projects.svelte.js';

	type Order = { id: number; status: OrderStatus; created_at: string; prize: string };

	let orders = $state<Order[] | null>(null);
	let broken = $state(false);
	let picks = $state<Prize[] | null>(null);
	let browsing = $state(false);
	let claims = $state<Record<number, number>>({});
	let chosen = $state<Prize | null>(null);
	let confirming = $state(false);
	let error = $state('');
	let claiming = $state(false);

	onMount(() => {
		load();
		if (!projects.hackatime) link();
		if (!catalog.loaded) browse();
		refresh();
	});

	function refresh() {
		return fetch('/api/orders', { cache: 'no-cache' })
			.then((response) => (response.ok ? response.json() : Promise.reject()))
			.then((data) => ((orders = data.orders), (claims = data.claims), (broken = false)))
			.catch(() => (broken = true));
	}

	function choose(prize: Prize) {
		chosen = prize;
		error = '';
		confirming = true;
	}

	async function claim() {
		if (!chosen || claiming) return;
		claiming = true;
		try {
			await request('POST', '/api/orders', { prize: chosen.id });
			confirming = false;
			await refresh();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		claiming = false;
	}

	function draw(list: NonNullable<typeof projects.list>) {
		const pool = [...catalog.prizes];
		const take = (rings?: number[]) => {
			const matching = rings ? pool.filter((prize) => rings.includes(prize.ring)) : pool;
			const from = matching.length ? matching : pool;
			const prize = from[Math.floor(Math.random() * from.length)];
			if (prize) pool.splice(pool.indexOf(prize), 1);
			return prize;
		};
		const held = list.map(ring);
		const best = Math.min(...held);
		const drawn = held.length ? [take(held), take(held), take([Math.max(best - 1, 0)]), take([0])] : [take(), take(), take(), take()];
		return drawn.filter(Boolean);
	}

	$effect(() => {
		if (!picks && catalog.loaded && (projects.list || projects.broken) && projects.hackatime) picks = draw(projects.list ?? []);
	});
</script>

<div class="prizes">
	<section>
		{#if catalog.broken}
			<p class="muted">Couldn't load the prizes</p>
		{:else if !picks}
			<p class="muted">Loading the prizes...</p>
		{:else if !picks.length}
			<p class="muted">No prizes yet</p>
		{:else}
			<div class="picks">
				{#each picks as prize (prize.id)}<PrizeCard {prize} />{/each}
			</div>
		{/if}
		<button class="button" disabled={!catalog.loaded} onclick={() => (browsing = true)}>See all prizes</button>
	</section>

	<section>
		<h3>Track Orders</h3>
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
	</section>
</div>

<Dialog bind:open={browsing} title="All prizes" wide>
	<AllPrizes {claims} onclaim={choose} />
</Dialog>

<Dialog bind:open={confirming} title="Claim prize">
	<p>Claim {chosen?.name}? It will be shipped to the address on your Hack Club account.</p>
	{#if error}<p class="error">{error}</p>{/if}
	<footer>
		<button class="button" onclick={() => (confirming = false)}>Cancel</button>
		<button class="button primary" disabled={claiming} onclick={claim}>{claiming ? 'Claiming...' : 'Claim'}</button>
	</footer>
</Dialog>

<style>
	.prizes {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	section > .button {
		align-self: flex-start;
	}

	.picks {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: 8px;
	}

	h3 {
		margin: 0;
		font: 18px 'Share Tech', sans-serif;
	}

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

	p {
		margin: 0 0 16px;
	}

	.error {
		color: var(--negative);
		font-size: 12px;
	}

	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
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
