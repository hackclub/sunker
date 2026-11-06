<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Prize } from '#lib/contracts.svelte.js';

	let { prize, children }: { prize: Prize; children?: Snippet } = $props();
</script>

<article class="prize">
	<div class="picture" class:blank={!prize.image}>
		{#if prize.image}
			<img src={prize.image} alt="" onerror={(event) => ((event.currentTarget as HTMLElement).hidden = true)} />
		{/if}
	</div>
	<span class="ring">Ring {prize.ring}</span>
	<h4>{prize.name}</h4>
	{#if prize.tagline}<p class="tagline">{prize.tagline}</p>{/if}
	{@render children?.()}
</article>

<style>
	.prize {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		padding: 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	.picture {
		display: grid;
		place-items: center;
		aspect-ratio: 4 / 3;
		margin-bottom: 4px;
		border-radius: 4px;
		overflow: hidden;
	}

	.blank {
		background: var(--background);
	}

	img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.ring {
		color: var(--accent);
		font-size: 11px;
	}

	h4 {
		margin: 0;
		font: 16px 'Share Tech', sans-serif;
	}

	.tagline {
		margin: 0;
		color: var(--text-muted);
		font-size: 12px;
		font-style: italic;
	}
</style>
