<svelte:head>
	<title>Hack Club Sunker</title>
</svelte:head>

<main>
	<h1>Hack Club Sunker</h1>
	<a href="/hq">Enter HQ</a>
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
		color: var(--accent);
	}
</style>
