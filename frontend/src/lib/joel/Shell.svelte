<script lang="ts">
	import type { Snippet } from 'svelte';

	let { map, tools, panel }: { map: Snippet; tools?: Snippet; panel: Snippet } = $props();
</script>

<div class="joel">
	<section class="stage">
		{@render map()}
		{#if tools}<div class="tools">{@render tools()}</div>{/if}
	</section>
	<aside>{@render panel()}</aside>
</div>

<style>
	.joel {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 400px;
		gap: 8px;
		height: calc(100% + 48px);
		margin: -16px -16px -32px;
	}

	.stage {
		position: relative;
		min-height: 0;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--background);
	}

	.tools {
		position: absolute;
		top: 12px;
		left: 12px;
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		max-width: calc(100% - 24px);
	}

	aside {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-height: 0;
		padding: 16px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		overflow-y: auto;
	}

	.joel :global(:is(.button, .chip)) {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.joel :global(.button:hover:not(:disabled)) {
		background: var(--hover);
	}

	.joel :global(.button:disabled) {
		opacity: 0.5;
		cursor: default;
	}

	.joel :global(.button.primary) {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--background);
	}

	.joel :global(.button.danger) {
		color: var(--negative);
	}

	.joel :global(.button.primary.danger) {
		border-color: var(--negative);
		background: var(--negative);
		color: var(--background);
	}

	.joel :global(.button[aria-pressed='true']) {
		border-color: var(--accent);
		color: var(--accent);
	}

	.joel :global(.badge) {
		padding: 1px 6px;
		border: 1px solid var(--island-border);
		border-radius: 4px;
		color: var(--text-muted);
		font-size: 11px;
		white-space: nowrap;
	}

	.joel :global(.badge.draft) {
		color: var(--text);
	}

	.joel :global(.badge.scheduled) {
		border-color: var(--flag);
		color: var(--flag);
	}

	.joel :global(.badge.resolved) {
		border-color: var(--positive);
		color: var(--positive);
	}

	.joel :global(.badge.negative),
	.joel :global(.badge.cancelled),
	.joel :global(.badge.rolled_back) {
		border-color: var(--negative);
		color: var(--negative);
	}

	.joel :global(h2) {
		margin: 0 0 8px;
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.joel :global(:is(input[type='text'], input[type='datetime-local'], textarea)) {
		width: 100%;
		box-sizing: border-box;
		padding: 6px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	.joel :global(textarea) {
		resize: vertical;
	}

	.joel :global(.muted) {
		color: var(--text-muted);
		font-size: 12px;
	}

	.joel :global(.error) {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
		white-space: pre-line;
	}

	.joel :global(.row) {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
	}

	.joel :global(.card) {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--background);
	}
</style>
