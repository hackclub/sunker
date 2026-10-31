<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		description,
		onclose,
		children
	}: { title: string; description?: string; onclose: () => void; children: Snippet } = $props();

	let dialog: HTMLDialogElement;

	$effect(() => dialog.showModal());
</script>

<dialog
	bind:this={dialog}
	{onclose}
	onclick={(event) => event.target === dialog && dialog.close()}
	aria-labelledby="dialog-title"
>
	<div class="body">
		<header>
			<div>
				<h2 id="dialog-title">{title}</h2>
				{#if description}<p>{description}</p>{/if}
			</div>
			<button class="close" aria-label="Close" onclick={() => dialog.close()}>
				<svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
					<path d="M2 2L12 12M12 2L2 12" />
				</svg>
			</button>
		</header>
		{@render children()}
	</div>
</dialog>

<style>
	dialog {
		width: min(960px, calc(100vw - 32px));
		max-height: calc(100vh - 32px);
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		color: var(--text);
		font-family: 'Inter', sans-serif;
		box-shadow: 0 16px 48px var(--shadow);
	}

	dialog::backdrop {
		background: rgb(0 0 0 / 0.6);
	}

	.body {
		display: flex;
		flex-direction: column;
		gap: 20px;
		padding: 20px 24px 24px;
	}

	header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 16px;
	}

	h2 {
		margin: 0;
		font-size: 18px;
		font-weight: 500;
	}

	p {
		margin: 4px 0 0;
		color: var(--text-muted);
		font-size: 13px;
	}

	.close {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border: none;
		border-radius: 6px;
		background: none;
		cursor: pointer;
	}

	.close:hover {
		background: var(--hover);
	}

	.close path {
		stroke: var(--text-muted);
		stroke-width: 1.5;
		stroke-linecap: round;
	}
</style>
