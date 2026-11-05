<script lang="ts">
	import { page } from '$app/state';
	import { allowedTabs } from '#lib/admin/access.js';
	import Konami from '#lib/Konami.svelte';
	import type { LayoutProps } from './$types';

	let { data, children }: LayoutProps = $props();

	const tabs = $derived(allowedTabs(data.admin));

	let open = $state(false);
	let menu: HTMLElement;

	$effect(() => {
		document.documentElement.style.overscrollBehavior = 'none';
		return () => document.documentElement.style.removeProperty('overscroll-behavior');
	});

	$effect(() => {
		page.url.pathname;
		open = false;
	});
</script>

<svelte:window
	onpointerdown={(event) => open && !menu.contains(event.target as Node) && (open = false)}
	onkeydown={(event) => event.key === 'Escape' && (open = false)}
/>

<Konami enabled={data.admin.superadmin} />

<div class="admin">
	<div class="menu" bind:this={menu}>
		<button class="burger" aria-label="Menu" aria-expanded={open} aria-controls="admin-nav" onclick={() => (open = !open)}>
			<svg viewBox="0 0 16 16"><path d="M2 4h12M2 8h12M2 12h12" /></svg>
		</button>
		<nav id="admin-nav" class:open>
			<a class="home" href="/hq" onclick={() => (open = false)}>Hack Club Sunker</a>
			{#each tabs as tab (tab.slug)}
				<a
					href="/admin/{tab.slug}"
					aria-current={page.url.pathname.startsWith(`/admin/${tab.slug}`) ? 'page' : undefined}
					onclick={() => (open = false)}
				>
					{tab.name}
				</a>
			{/each}
		</nav>
	</div>
	<main>
		{@render children()}
	</main>
</div>

<style>
	.admin {
		display: flex;
		gap: 8px;
		padding: max(8px, env(safe-area-inset-top)) max(8px, env(safe-area-inset-right)) max(8px, env(safe-area-inset-bottom)) max(8px, env(safe-area-inset-left));
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
		overflow-y: auto;
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
		overscroll-behavior: contain;
	}

	.menu {
		display: contents;
	}

	.burger {
		display: none;
	}

	@media (max-width: 900px), (max-height: 480px) {
		.admin {
			padding-bottom: max(8px, env(safe-area-inset-bottom));
		}

		.menu {
			display: block;
			position: fixed;
			right: calc(16px + env(safe-area-inset-right));
			bottom: calc(16px + env(safe-area-inset-bottom));
			z-index: 30;
		}

		.burger {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 56px;
			height: 56px;
			padding: 0;
			border: 1px solid var(--island-border);
			border-radius: 16px;
			background: var(--island);
			color: var(--text);
			box-shadow: 0 4px 16px var(--shadow);
			cursor: pointer;
		}

		.burger[aria-expanded='true'] {
			color: var(--accent);
		}

		.burger svg {
			width: 24px;
			height: 24px;
			fill: none;
			stroke: currentColor;
			stroke-width: 1.5;
			stroke-linecap: round;
		}

		nav {
			display: none;
			position: absolute;
			right: 0;
			bottom: 64px;
			box-sizing: border-box;
			width: max-content;
			min-width: 200px;
			max-height: calc(100dvh - 96px - env(safe-area-inset-top) - env(safe-area-inset-bottom));
			padding: 8px;
			overflow-y: auto;
			overscroll-behavior: contain;
			box-shadow: 0 12px 32px var(--shadow);
		}

		nav.open {
			display: flex;
		}

		a {
			display: flex;
			align-items: center;
			min-height: 44px;
			padding: 0 12px;
		}

		.home {
			margin-bottom: 4px;
		}

		main {
			--main-pad-top: 12px;
			--main-pad-x: 12px;
			--main-pad-bottom: 16px;
			padding: 12px 12px 88px;
		}
	}
</style>
