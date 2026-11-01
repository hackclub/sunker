<script lang="ts">
	import { onMount } from 'svelte';
	import { params } from '#lib/url.svelte.js';
	import { place, type Force } from '#lib/map.js';
	import { key, regions, request, type JoelState } from '#lib/joel/joel.js';
	import Overview from '#lib/joel/Overview.svelte';
	import Workspace from '#lib/joel/Workspace.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let joel = $state<JoelState | null>(null);
	let view = $state<{ x: number; y: number; scale: number } | null>(null);
	let error = $state('');

	const tiles = $derived(place(data.map));
	const placed = $derived(new Set(tiles.map((tile) => key(tile.row, tile.col))));
	const names = $derived(regions(data.map));
	const enemies = $derived(new Map<string, Force>((joel?.territory ?? []).map((tile) => [key(tile.row, tile.col), tile.enemies])));
	const open = $derived(joel?.waves.find((wave) => String(wave.id) === params().get('wave')));

	async function refresh() {
		try {
			joel = await request('GET', '/api/admin/joel');
			error = '';
		} catch (e) {
			error = `Couldn't load Joel View: ${e instanceof Error ? e.message : e}`;
		}
	}

	onMount(() => {
		refresh();
		const timer = setInterval(refresh, 30000);
		return () => clearInterval(timer);
	});
</script>

<svelte:head>
	<title>Joel View - Hack Club Sunker Admin</title>
</svelte:head>

{#if !joel}
	<p class="status">{error || 'Loading Joel View...'}</p>
{:else if open}
	{#key open.id}
		<Workspace wave={open} {joel} {tiles} {placed} {names} {enemies} bind:view {refresh} />
	{/key}
{:else}
	<Overview {joel} {tiles} {placed} {names} {enemies} bind:view {refresh} />
{/if}

<style>
	.status {
		margin: 0;
		color: var(--text-muted);
	}
</style>
