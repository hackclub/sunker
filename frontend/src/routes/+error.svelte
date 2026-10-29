<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
</script>

<main>
	<h1>{page.status}</h1>
	<p>{page.error?.message}</p>
	<button onclick={() => invalidateAll()}>Retry</button>
</main>

<style>
	main {
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 12px;
		position: fixed;
		inset: 0;
		background: var(--background);
		color: var(--text);
		font-family: 'Inter', sans-serif;
	}

	h1 {
		margin: 0;
		color: var(--accent);
		font-size: 48px;
	}

	p {
		margin: 0;
	}

	button {
		margin-top: 8px;
		padding: 8px 20px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
</style>
