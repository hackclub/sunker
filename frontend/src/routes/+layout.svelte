<script lang="ts">
	import { PALETTE } from '#lib/colors.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	const variables = Object.entries(PALETTE)
		.map(([name, value]) => `--${name}: ${value};`)
		.join(' ');
</script>

<svelte:head>
	<link rel="icon" href="/favicon.png" />
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Belgrano&family=Inter:wght@400;500&display=swap" />
	{@html `<style>:root { ${variables} }</style>`}
</svelte:head>

{@render children()}
