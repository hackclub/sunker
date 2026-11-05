<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';

	onMount(() => {
		let visitor: string = crypto.randomUUID();
		try {
			visitor = localStorage.getItem('visitor') ?? visitor;
			localStorage.setItem('visitor', visitor);
		} catch {}
		fetch('/api/visit', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ visitor })
		}).catch(() => {});
	});
</script>

<svelte:head>
	<title>Hack Club Sunker</title>
</svelte:head>

<main>
	<h1>Hack Club Sunker</h1>
	<a href="/login{page.url.search}" rel="external">Enter HQ</a>
</main>

<style>
	main {
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 16px;
		position: fixed;
		inset: 0;
		background: var(--background);
		color: var(--text);
		font-family: 'Inter', sans-serif;
	}

	h1 {
		margin: 0;
		font-weight: 500;
	}

	a {
		position: relative;
		color: var(--accent);
	}

	a::after {
		content: '';
		position: absolute;
		inset: -14px -20px;
	}
</style>
