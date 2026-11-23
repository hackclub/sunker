<script lang="ts">
	import { Spring } from 'svelte/motion';
	import { image, rendering, styles } from '#lib/styles.svelte.js';

	let { tilt = 0, mode = 'float', size, onclick }: { tilt?: number; mode?: 'float' | 'bob'; size: string; onclick?: () => void } = $props();

	const turn = new Spring(0, { stiffness: 0.12, damping: 0.28 });
	const head = $derived((styles.images['robosquirrel-head']?.width ?? 128) / (styles.images['robosquirrel-body']?.width ?? 256));
	$effect(() => {
		turn.target = tilt;
	});
</script>

{#snippet parts()}
	<img class="body" src={image('robosquirrel-body')} alt="" style:image-rendering={rendering('robosquirrel-body')} />
	<span class="neck" class:bob={mode === 'bob'} style:--head={head}>
		<img class="head" src={image('robosquirrel-head')} alt="" style:image-rendering={rendering('robosquirrel-head')} style:transform="rotate({turn.current}deg)" />
	</span>
{/snippet}

{#if onclick}
	<button class="robosquirrel" class:float={mode === 'float'} style:--size={size} aria-label="Robosquirrel" {onclick}>{@render parts()}</button>
{:else}
	<div class="robosquirrel" class:float={mode === 'float'} style:--size={size}>{@render parts()}</div>
{/if}

<style>
	.robosquirrel {
		position: relative;
		display: block;
		width: var(--size);
		aspect-ratio: 1;
		padding: 0;
		border: none;
		background: none;
	}

	button.robosquirrel {
		cursor: pointer;
	}

	.float {
		animation: float 4s ease-in-out infinite;
	}

	img {
		position: absolute;
		pointer-events: none;
	}

	.body {
		inset: 0;
		width: 100%;
	}

	.neck {
		position: absolute;
		top: calc(41% - var(--head) * 90%);
		left: calc(65% - var(--head) * 50%);
		width: calc(var(--head) * 100%);
		aspect-ratio: 1;
	}

	.bob {
		animation: bob 3.2s ease-in-out infinite;
	}

	.head {
		inset: 0;
		width: 100%;
		transform-origin: 50% 90%;
	}

	@keyframes float {
		50% {
			transform: translateY(-6%);
		}
	}

	@keyframes bob {
		50% {
			transform: translateY(5%);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.float,
		.bob {
			animation: none;
		}
	}
</style>
