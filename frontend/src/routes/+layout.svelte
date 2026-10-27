<script lang="ts">
	import favicon from '#lib/assets/favicon.svg';
	import MatrixBackground from '#lib/MatrixBackground.svelte';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<MatrixBackground />

{@render children()}

<style>
	@import url('https://fonts.googleapis.com/css2?family=Share+Tech&family=JetBrains+Mono:ital,wght@0,100..800;1,100..800&display=swap');

	:global(:root) {
		--bg: #0e0f10;
		--island: #1e1f22;
		--island-border: #2b2d30;
		--hover: #2b2d30;
		--text: #dfe1e5;
		--text-muted: #868a91;
		--accent: #8bcf86;
		--radius: 12px;
		--radius-small: 6px;
		color-scheme: dark;
	}

	:global(body) {
		margin: 0;
		background: var(--bg);
		color: var(--text);
		font-family: 'Share Tech', sans-serif;
	}

	:global(:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
</style>
