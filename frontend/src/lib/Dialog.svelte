<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		open = $bindable(false),
		title,
		wide = false,
		children
	}: { open?: boolean; title: string; wide?: boolean; children?: Snippet } = $props();

	let dialog: HTMLDialogElement;

	$effect(() => {
		if (open && !dialog.open) dialog.showModal();
		if (!open && dialog.open) dialog.close();
	});
</script>

<dialog class:wide bind:this={dialog} onclose={() => (open = false)}>
	<header>
		<h2>{title}</h2>
		<button aria-label="Close" onclick={() => (open = false)}>
			<svg viewBox="0 0 16 16" aria-hidden="true">
				<path d="M4 4l8 8M12 4l-8 8" />
			</svg>
		</button>
	</header>
	<div class="content">
		{@render children?.()}
	</div>
</dialog>

<style>
	dialog {
		width: min(440px, calc(100vw - 32px));
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		box-shadow: 0 16px 48px var(--shadow);
	}

	dialog.wide {
		width: min(960px, calc(100vw - 32px));
		height: calc(100dvh - 64px);
	}

	dialog.wide[open] {
		display: flex;
		flex-direction: column;
	}

	.wide .content {
		flex: 1;
		min-height: 0;
		overflow: auto;
		overscroll-behavior: contain;
	}

	@media (max-width: 600px), (max-height: 480px) {
		dialog.wide {
			width: 100vw;
			max-width: none;
			height: 100dvh;
			max-height: none;
			border: none;
			border-radius: 0;
		}

		.wide header {
			padding-top: calc(12px + env(safe-area-inset-top, 0px));
		}

		.wide .content {
			padding-bottom: calc(20px + env(safe-area-inset-bottom, 0px));
		}
	}

	dialog::backdrop {
		background: var(--shadow);
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 12px 12px 20px;
		border-bottom: 1px solid var(--island-border);
	}

	h2 {
		margin: 0;
		font: 18px 'Share Tech', sans-serif;
	}

	.content {
		padding: 16px 20px 20px;
		font: 13px/1.5 'Inter', sans-serif;
	}

	header button {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}

	header button:hover {
		background: var(--hover);
		color: var(--text);
	}

	@media (pointer: coarse) {
		header button {
			width: 44px;
			height: 44px;
			margin: -8px -6px;
		}
	}

	svg {
		width: 16px;
		height: 16px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.5;
		stroke-linecap: round;
	}
</style>
