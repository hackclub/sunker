<script lang="ts">
	import { DEFAULTS, colour } from '#lib/colors.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	const variables = $derived(
		Object.keys(DEFAULTS)
			.map((key) => `--${key}: ${colour(key)};`)
			.join(' ')
	);
</script>

<svelte:head>
	<link rel="icon" href="/favicon.png" />
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Belgrano&family=Inter:wght@400;500&family=JetBrains+Mono&family=Share+Tech&display=swap" />
	{@html `<style>:root { ${variables} }</style>`}
</svelte:head>

{@render children()}

<style>
	:global(button, .button, [role='button']) {
		user-select: none;
	}
</style>
