<script lang="ts">
	import type { Snippet } from 'svelte';

	let { side, children }: { side: 'left' | 'right'; children?: Snippet } = $props();

	const WIDTH = 64;
	const DROP = WIDTH / Math.sqrt(3);

	let height = $state(0);
</script>

<div class="panel {side}" style:--width="{WIDTH}px" style:--drop="{DROP}px" bind:clientHeight={height}>
	<svg width={WIDTH} {height}>
		<polygon points="0,0 {WIDTH},{DROP} {WIDTH},{height - DROP} 0,{height}" />
	</svg>
	<div class="content">
		{@render children?.()}
	</div>
</div>

<style>
	.panel {
		position: fixed;
		top: 50%;
		width: var(--width);
		transform: translateY(-50%);
	}

	.left {
		left: 0;
	}

	.right {
		right: 0;
	}

	svg {
		position: absolute;
		inset: 0;
		overflow: visible;
	}

	.right svg {
		transform: scaleX(-1);
	}

	polygon {
		fill: var(--island);
		stroke: var(--island-border);
		stroke-width: 1;
	}

	.content {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 16px;
		padding: calc(var(--drop) + 16px) 0;
	}
</style>
