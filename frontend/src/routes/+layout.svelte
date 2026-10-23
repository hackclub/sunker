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

	:global(body) {
		background: #0e0f10;
		font-family: 'Share Tech', sans-serif;
	}
</style>
