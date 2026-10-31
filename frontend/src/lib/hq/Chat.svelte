<script lang="ts">
	import { params, update } from '#lib/url.svelte.js';

	type Chat = { id: number; name: string };

	let chats = $state<Chat[]>([]);
	const selected = $derived(chats.find((chat) => String(chat.id) === params().get('chat'))?.id ?? chats[0]?.id);
	let failed = $state(false);

	$effect(() => {
		fetch('/api/chats', { cache: 'no-cache' })
			.then((response) => {
				if (!response.ok) throw new Error(`${response.status}`);
				return response.json();
			})
			.then((list: Chat[]) => (chats = list))
			.catch(() => (failed = true));
	});
</script>

{#if failed}
	<p class="error">Couldn't load chats</p>
{:else}
	<nav>
		{#each chats as chat (chat.id)}
			<button class:selected={chat.id === selected} onclick={() => update({ chat: chat.id })}>{chat.name}</button>
		{/each}
	</nav>
{/if}

<style>
	nav {
		display: flex;
		gap: 8px;
		overflow-x: auto;
	}

	nav button {
		flex: none;
		padding: 8px 16px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
	}

	nav button:hover {
		background: var(--hover);
		color: var(--text);
	}

	nav button.selected {
		border-color: var(--accent);
		color: var(--text);
	}

	.error {
		color: var(--text-muted);
	}
</style>
