<script lang="ts">
	import type { ChatMessage } from './Message.svelte';

	type Props = {
		onsend: (contents: string, reply: number | null) => Promise<void>;
		replying?: ChatMessage | null;
		oncancel?: () => void;
		placeholder?: string;
		slowmode?: number;
	};

	let { onsend, replying = null, oncancel, placeholder = 'Send message', slowmode = 0 }: Props = $props();

	let contents = $state('');
	let sending = $state(false);
	let waiting = $state(0);
	let field: HTMLTextAreaElement;

	function cool() {
		waiting = slowmode / 1000;
		const timer = setInterval(() => --waiting || clearInterval(timer), 1000);
	}

	$effect(() => {
		if (replying) field.focus();
	});

	async function submit() {
		const text = contents.trim();
		if (!text || sending || waiting) return;
		sending = true;
		contents = '';
		try {
			await onsend(text, replying?.id ?? null);
			if (slowmode) cool();
		} catch {
			if (!contents.trim()) contents = text;
		} finally {
			sending = false;
		}
	}
</script>

<form
	onsubmit={(event) => {
		event.preventDefault();
		submit();
	}}
>
	{#if replying}
		<div class="replying">
			<span>Replying to <strong>{replying.author.handle ?? 'Anonymous'}</strong></span>
			<button type="button" aria-label="Cancel reply" onclick={oncancel}>
				<svg viewBox="0 0 16 16"><path d="M4 4l8 8M12 4l-8 8" /></svg>
			</button>
		</div>
	{/if}
	<hr />
	<textarea
		bind:this={field}
		bind:value={contents}
		{placeholder}
		rows="1"
		maxlength="2000"
		enterkeyhint="send"
		onkeydown={(event) => {
			if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
				event.preventDefault();
				submit();
			} else if (event.key === 'Escape' && replying) {
				event.stopPropagation();
				oncancel?.();
			}
		}}
	></textarea>
	<button class="send" type="submit" aria-label={waiting ? `Slowmode, ${waiting}s left` : 'Send'} disabled={sending || waiting > 0 || !contents.trim()}>
		{#if waiting}
			<span class="wait">{waiting}</span>
		{:else}
			<svg viewBox="0 0 16 16"><path d="M14.5 1.5 7 9M14.5 1.5 10 14.5 7 9 1.5 6z" /></svg>
		{/if}
	</button>
</form>

<style>
	form {
		display: flex;
		flex-direction: column;
		flex: none;
		padding-top: 8px;
	}

	.replying {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 0 6px 12px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.replying strong {
		color: var(--text);
		font-weight: 500;
	}

	hr {
		margin: 0 0 0 var(--line-start, 0);
		border: none;
		border-top: 1px solid var(--island-border);
	}

	textarea {
		max-height: 160px;
		padding: 10px 12px 4px;
		border: none;
		outline: none;
		background: transparent;
		color: var(--text);
		font: inherit;
		resize: none;
		field-sizing: content;
	}

	textarea::placeholder {
		color: var(--text-muted);
		opacity: 0.6;
	}

	button {
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

	button:hover:not(:disabled) {
		background: var(--hover);
		color: var(--text);
	}

	.send {
		align-self: flex-end;
	}

	.send:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.wait {
		font-size: 12px;
		font-variant-numeric: tabular-nums;
	}

	@media (pointer: coarse) {
		form {
			display: grid;
			grid-template-columns: minmax(0, 1fr) auto;
			align-items: end;
		}

		.replying,
		hr {
			grid-column: 1 / -1;
		}

		.replying button {
			margin: -8px 0;
		}

		textarea {
			box-sizing: border-box;
			min-height: 44px;
			padding: 12px 4px 12px 12px;
		}

		button {
			width: 44px;
			height: 44px;
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
