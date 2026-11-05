<script lang="ts" module>
	export type Reply = { id: number; contents: string | null; handle: string | null; deleted: boolean };

	export type ChatMessage = {
		id: number;
		author: { handle: string | null };
		contents: string;
		created_at: string;
		deleted_at?: string | null;
		reply_to: Reply | null;
	};
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = {
		message: ChatMessage;
		flagged?: boolean;
		flashing?: boolean;
		onflag?: () => void;
		onreply?: () => void;
		onjump?: (id: number) => void;
		ondelete?: () => void;
		meta?: Snippet;
	};

	let { message, flagged = false, flashing = false, onflag, onreply, onjump, ondelete, meta }: Props = $props();

	const time = $derived(new Date(message.created_at).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }));

	let root: HTMLElement;
	let picked = $state(false);
</script>

<svelte:window onpointerdown={(event) => picked && event.target instanceof Node && !root.contains(event.target) && (picked = false)} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<article
	bind:this={root}
	data-message={message.id}
	class:picked
	onclick={(event) => event.target instanceof Element && !event.target.closest('.quote, .tools') && (picked = !picked)}
	class:flagged
	class:flashing
	class:reply={message.reply_to}
	class:deleted={message.deleted_at}
>
	{#if message.reply_to}
		{@const original = message.reply_to}
		<button class="quote" disabled={original.contents === null} onclick={() => onjump?.(original.id)}>
			{#if original.contents === null}
				Original message deleted
			{:else}
				<span class="handle">{original.handle ?? 'Anonymous'}</span>
				{#if original.deleted}<em>(deleted)</em>{/if}
				{original.contents}
			{/if}
		</button>
	{/if}
	<header>
		<span class="handle">{message.author.handle ?? 'Anonymous'}</span>
		{@render meta?.()}
		<time datetime={message.created_at}>{time}</time>
	</header>
	<p>{message.contents}</p>
	{#if onflag || onreply || ondelete}
		<div class="tools">
			{#if onreply}
				<button aria-label="Reply" title="Reply" onclick={onreply}>
					<svg viewBox="0 0 16 16"><path d="M6.5 3.5 2.5 7.5l4 4M2.5 7.5h7a4 4 0 0 1 4 4v1" /></svg>
				</button>
			{/if}
			{#if onflag}
				<button class:active={flagged} aria-label={flagged ? 'Unflag' : 'Flag'} title={flagged ? 'Unflag' : 'Flag'} onclick={onflag}>
					<svg viewBox="0 0 16 16"><path d="M3.5 14.5v-12M3.5 2.5h8l-2 3 2 3h-8" /></svg>
				</button>
			{/if}
			{#if ondelete}
				<button aria-label="Delete" title="Delete" onclick={ondelete}>
					<svg viewBox="0 0 16 16"><path d="M2.5 4.5h11M6.5 4.5v-2h3v2M4 4.5l.75 9h6.5L12 4.5M6.75 7v4.5M9.25 7v4.5" /></svg>
				</button>
			{/if}
		</div>
	{/if}
</article>

<style>
	article {
		position: relative;
		padding: 6px 12px;
		border-radius: 6px;
		transition: background 0.6s;
	}

	article:hover {
		background: var(--hover);
	}

	article.flagged {
		background: color-mix(in srgb, var(--flag) 22%, transparent);
	}

	article.flashing {
		background: color-mix(in srgb, var(--accent) 30%, transparent);
		transition: none;
	}

	article.reply {
		padding-left: 30px;
	}

	article.reply::before {
		content: '';
		position: absolute;
		top: 14px;
		bottom: 10px;
		left: 12px;
		width: 10px;
		border: 2px solid var(--tip-border);
		border-right: none;
		border-radius: 8px 0 0 8px;
	}

	article.deleted p {
		color: var(--text-muted);
		text-decoration: line-through;
	}

	.quote {
		display: block;
		max-width: 100%;
		margin-bottom: 2px;
		padding: 0;
		overflow: hidden;
		border: none;
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: 12px;
		text-align: left;
		text-overflow: ellipsis;
		white-space: nowrap;
		opacity: 0.7;
		cursor: pointer;
	}

	.quote:hover:not(:disabled) {
		opacity: 1;
	}

	.quote:disabled {
		font-style: italic;
		cursor: default;
	}

	header {
		display: flex;
		align-items: baseline;
		gap: 8px;
		flex-wrap: wrap;
	}

	.handle {
		color: var(--text);
		font-weight: 500;
	}

	.quote .handle {
		color: inherit;
		margin-right: 4px;
	}

	time {
		color: var(--text-muted);
		font-size: 11px;
	}

	p {
		margin: 2px 0 0;
		color: var(--text);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.tools {
		display: flex;
		position: absolute;
		top: 4px;
		right: 6px;
		gap: 2px;
		padding: 2px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		opacity: 0;
		pointer-events: none;
	}

	article:hover .tools,
	.tools:focus-within {
		opacity: 1;
		pointer-events: auto;
	}

	.tools button {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		padding: 0;
		border: none;
		border-radius: 4px;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}

	.tools button:hover {
		background: var(--hover);
		color: var(--text);
	}

	.tools button.active {
		color: var(--flag);
	}

	@media (hover: none) {
		article.picked .tools {
			opacity: 1;
			pointer-events: auto;
		}

		.tools button {
			width: 40px;
			height: 40px;
		}
	}

	svg {
		width: 16px;
		height: 16px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
