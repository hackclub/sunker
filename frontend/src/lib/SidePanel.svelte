<script module lang="ts">
	const WIDTH = 64;
	const GAP = 16;
	const DROP = WIDTH / Math.sqrt(3);
	const RADIUS = 6;
	export const DIALOG_WIDTH = 480;
	export const SPLIT = 1200;

	const span = (count: number) => count * WIDTH + (count - 1) * GAP + 2 * (DROP + GAP);
	export const offset = (count: number, index: number) => DROP + GAP + index * (WIDTH + GAP) - span(count) / 2;
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	let { side, open = false, peek = false, count, dialog }: { side: 'left' | 'right'; open?: boolean; peek?: boolean; count: number; dialog?: Snippet } = $props();

	const height = $derived(span(count));

	function toward([px, py]: number[], [qx, qy]: number[]) {
		const len = Math.hypot(qx - px, qy - py);
		return `${px + ((qx - px) * RADIUS) / len} ${py + ((qy - py) * RADIUS) / len}`;
	}

	const outline = $derived.by(() => {
		const top = [WIDTH, DROP];
		const bottom = [WIDTH, height - DROP];
		return [
			'M 0 0',
			`L ${toward(top, [0, 0])} Q ${top.join(' ')} ${toward(top, bottom)}`,
			`L ${toward(bottom, top)} Q ${bottom.join(' ')} ${toward(bottom, [0, height])}`,
			`L 0 ${height}`
		].join(' ');
	});
</script>

<aside class="dialog {side}" class:open class:peek style:--dialog-width="{DIALOG_WIDTH}px" inert={!open}>
	{@render dialog?.()}
</aside>

<div
	class="panel {side}"
	class:open
	style:--width="{WIDTH}px"
	style:--dialog-width="{DIALOG_WIDTH}px"
	style:--height="{height}px"
>
	<svg width={WIDTH} {height}>
		<path class="fill" d="{outline} Z" />
		<path class="border" d={outline} />
	</svg>
</div>

<style>
	.panel {
		--shift: 0px;
		position: fixed;
		top: 50%;
		z-index: 20;
		width: var(--width);
		height: var(--height);
		transition: transform 0.25s ease;
	}

	.panel.open {
		--shift: calc(min(var(--dialog-width), 100vw - 144px) + 8px);
	}

	.panel.left {
		left: 0;
		transform: translate(var(--shift), -50%);
	}

	.panel.right {
		right: 0;
		transform: translate(calc(-1 * var(--shift)), -50%);
	}

	svg {
		position: absolute;
		inset: 0;
		overflow: visible;
	}

	.right svg {
		transform: scaleX(-1);
	}

	.fill {
		fill: var(--island);
	}

	.border {
		fill: none;
		stroke: var(--background);
		stroke-width: 2;
	}

	.dialog {
		position: fixed;
		top: calc(8px + var(--vv-top, 0px));
		bottom: calc(8px + var(--vv-bottom, 0px));
		z-index: 20;
		box-sizing: border-box;
		width: var(--dialog-width);
		overflow: auto;
		overscroll-behavior: contain;
		border: 2px solid var(--background);
		border-radius: 6px;
		background: var(--island);
		transition: transform 0.25s ease;
	}

	.dialog.left {
		left: 8px;
		border-right: none;
		transform: translateX(calc(-100% - 8px));
	}

	.dialog.right {
		right: 8px;
		border-left: none;
		transform: translateX(calc(100% + 8px));
	}

	.dialog.open {
		transform: none;
	}

	@media (pointer: coarse), (orientation: portrait) and (max-width: 600px) {
		.dialog {
			display: flex;
			flex-direction: column;
		}
	}

	.dialog {
		width: min(var(--dialog-width), 100vw - 144px);
	}

	@media (orientation: portrait) and (max-width: 600px) {
		.panel {
			top: auto;
			bottom: 0;
			width: 50%;
			height: var(--bar, 0px);
			filter: drop-shadow(2px 0 var(--background)) drop-shadow(-2px 0 var(--background)) drop-shadow(0 2px var(--background))
				drop-shadow(0 -2px var(--background));
		}

		.panel.left,
		.panel.right {
			transform: none;
		}

		.panel svg {
			display: none;
		}

		.panel::before {
			content: '';
			position: absolute;
			inset: 0;
			background: var(--island);
		}

		.panel.left::before {
			clip-path: polygon(0 0, calc(100% - 17px) 0, 100% 10px, 100% 100%, 0 100%);
		}

		.panel.right::before {
			clip-path: polygon(0 10px, 17px 0, 100% 0, 100% 100%, 0 100%);
		}

		.dialog {
			top: calc(max(8px, env(safe-area-inset-top, 0px)) + var(--vv-top, 0px));
			bottom: calc(max(var(--bar, 0px), var(--vv-bottom, 0px)) + 8px);
		}

		.dialog.left,
		.dialog.right {
			right: 8px;
			left: 8px;
			width: auto;
			border: 2px solid var(--background);
		}

		.dialog.peek {
			top: auto;
			height: max(260px, 40dvh);
		}
	}
</style>
