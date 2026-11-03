<script lang="ts">
	import { page } from '$app/state';
	import { allowedTabs } from '#lib/admin/access.js';
	import Konami from '#lib/Konami.svelte';
	import type { LayoutProps } from './$types';

	let { data, children }: LayoutProps = $props();

	const tabs = $derived(allowedTabs(data.admin));
</script>

<Konami enabled={data.admin.superadmin} />

<div class="admin">
	<nav>
		<a class="home" href="/hq">Hack Club Sunker</a>
		{#each tabs as tab (tab.slug)}
			<a
				href="/admin/{tab.slug}"
				aria-current={page.url.pathname.startsWith(`/admin/${tab.slug}`) ? 'page' : undefined}
			>
				{tab.name}
			</a>
		{/each}
	</nav>
	<main>
		{@render children()}
	</main>
</div>

<style>
	.admin {
		display: flex;
		gap: 8px;
		padding: 8px;
		position: fixed;
		inset: 0;
		background: var(--background);
		color: var(--text);
		font-family: 'Inter', sans-serif;
	}

	nav {
		display: flex;
		flex-direction: column;
		gap: 4px;
		width: 220px;
		flex-shrink: 0;
		padding: 16px 12px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
	}

	a {
		padding: 8px 12px;
		border-radius: 6px;
		color: var(--text-muted);
		text-decoration: none;
	}

	a:hover {
		background: var(--hover);
		color: var(--text);
	}

	a[aria-current='page'] {
		background: var(--hover);
		color: var(--accent);
	}

	.home {
		margin-bottom: 12px;
		color: var(--text);
		font-weight: 500;
	}

	main {
		flex: 1;
		padding: 16px 16px 32px;
		overflow: auto;
	}
</style>
