<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		open = $bindable(false),
		title,
		children
	}: { open?: boolean; title: string; children?: Snippet } = $props();

	let dialog: HTMLDialogElement;

	$effect(() => {
		if (open && !dialog.open) dialog.showModal();
		if (!open && dialog.open) dialog.close();
	});
</script>

<dialog bind:this={dialog} onclose={() => (open = false)}>
	<header>
		<h2>{title}</h2>
		<button aria-label="Close" onclick={() => (open = false)}>
			<svg viewBox="0 0 16 16" aria-hidden="true">
				<path d="M4 4l8 8M12 4l-8 8" />
			</svg>
		</button>
	</header>
	{@render children?.()}
</dialog>

<style>
	dialog {
		width: min(640px, calc(100vw - 32px));
		min-height: 320px;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: var(--radius);
		background: var(--island);
		color: var(--text);
		box-shadow: 0 16px 48px var(--shadow);
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
		font-size: 16px;
		font-weight: normal;
	}

	button {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: none;
		border-radius: var(--radius-small);
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}

	button:hover {
		background: var(--hover);
		color: var(--text);
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
