<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		side,
		open = false,
		children,
		dialog
	}: { side: 'left' | 'right'; open?: boolean; children?: Snippet; dialog?: Snippet } = $props();

	const WIDTH = 64;
	const DROP = WIDTH / Math.sqrt(3);
	const RADIUS = 6;
	const DIALOG_WIDTH = 480;

	let height = $state(0);

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

<aside class="dialog {side}" class:open style:--dialog-width="{DIALOG_WIDTH}px" inert={!open}>
	{@render dialog?.()}
</aside>

<div
	class="panel {side}"
	class:open
	style:--width="{WIDTH}px"
	style:--drop="{DROP}px"
	style:--dialog-width="{DIALOG_WIDTH}px"
	bind:clientHeight={height}
>
	<svg width={WIDTH} {height}>
		<path class="fill" d="{outline} Z" />
		<path class="border" d={outline} />
	</svg>
	<div class="content">
		{@render children?.()}
	</div>
</div>

<style>
	.panel {
		--shift: 0px;
		position: fixed;
		top: 50%;
		width: var(--width);
		transition: transform 0.25s ease;
	}

	.panel.open {
		--shift: calc(var(--dialog-width) + 8px);
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
		stroke: #0e0f10;
		stroke-width: 2;
	}

	.content {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 16px;
		padding: calc(var(--drop) + 16px) 0;
	}

	.dialog {
		position: fixed;
		top: 8px;
		bottom: 8px;
		box-sizing: border-box;
		width: var(--dialog-width);
		overflow: auto;
		border: 2px solid #0e0f10;
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
</style>
