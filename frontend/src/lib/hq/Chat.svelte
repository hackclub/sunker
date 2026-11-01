<script lang="ts" module>
	const PLACEHOLDERS = [
		'Speak to the masses...',
		'Share your ponderings...',
		'Blabber loudly...',
		'Shout out into the void...',
		'Pitter-patter chipper-chatter...',
		'Come tarnished and speak...',
		'I have friends everywhere, let us converse...',
		"Get yappin'..."
	];

	let last: string | undefined;

	function pick() {
		const options = PLACEHOLDERS.filter((phrase) => phrase !== last);
		last = options[Math.floor(Math.random() * options.length)];
		return last;
	}
</script>

<script lang="ts">
	import { tick } from 'svelte';
	import { params, update } from '#lib/url.svelte.js';
	import Composer from '#lib/chat/Composer.svelte';
	import Message, { type ChatMessage } from '#lib/chat/Message.svelte';

	type Chat = { id: number; name: string };
	type Row = ChatMessage & { flagged: boolean };

	let chats = $state<Chat[]>([]);
	const selected = $derived(chats.find((chat) => String(chat.id) === params().get('chat'))?.id ?? chats[0]?.id);
	let failed = $state(false);
	let messages = $state<Row[]>([]);
	let replying = $state<Row | null>(null);
	let flashing = $state<number>();
	let scroller = $state<HTMLElement>();
	let root: HTMLElement;
	let placeholder = $state(pick());
	let shown = $state(false);
	let latest = 0;
	let around: number | undefined;
	const flagging = new Map<number, boolean>();

	$effect(() => {
		const panel = root.closest('aside');
		if (!panel) return;
		shown = !panel.inert;
		const observer = new MutationObserver(() => {
			if (!shown && !panel.inert) placeholder = pick();
			shown = !panel.inert;
		});
		observer.observe(panel, { attributes: true, attributeFilter: ['inert'] });
		return () => observer.disconnect();
	});

	$effect(() => {
		fetch('/api/chats', { cache: 'no-cache' })
			.then((response) => {
				if (!response.ok) throw new Error(`${response.status}`);
				return response.json();
			})
			.then((list: Chat[]) => (chats = list))
			.catch(() => (failed = true));
	});

	async function load(chat: number, scroll = false) {
		const request = ++latest;
		const response = await fetch(`/api/chats/${chat}/messages${around ? `?around=${around}` : ''}`, { cache: 'no-cache' });
		if (!response.ok || request !== latest || chat !== selected) return;
		const list: Row[] = await response.json();
		if (request !== latest || chat !== selected) return;
		for (const row of list) if (flagging.has(row.id)) row.flagged = flagging.get(row.id)!;
		const bottom = !scroller || scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 40;
		messages = list;
		await tick();
		if (scroll || bottom) scroller?.scrollTo({ top: scroller.scrollHeight });
	}

	$effect(() => {
		selected;
		messages = [];
		replying = null;
		around = undefined;
	});

	$effect(() => {
		const chat = selected;
		if (chat === undefined || !shown) return;
		const poll = (scroll = false) => load(chat, scroll).catch(() => {});
		poll(true);
		const timer = setInterval(() => document.hidden || poll(), 3000);
		return () => clearInterval(timer);
	});

	async function send(contents: string, reply: number | null) {
		const chat = selected;
		if (chat === undefined) return;
		const response = await fetch(`/api/chats/${chat}/messages`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ contents, reply_to: reply })
		});
		if (!response.ok) throw new Error(`${response.status}`);
		replying = null;
		placeholder = pick();
		load(chat, true).catch(() => {});
	}

	async function flag(message: Row) {
		const { id } = message;
		const flagged = !message.flagged;
		message.flagged = flagged;
		flagging.set(id, flagged);
		const response = await fetch(`/api/messages/${id}/flag`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ flagged })
		}).catch(() => null);
		flagging.delete(id);
		latest++;
		const row = messages.find((row) => row.id === id);
		if (row && !response?.ok) row.flagged = !flagged;
	}

	async function jump(id: number) {
		const chat = selected;
		if (chat === undefined) return;
		if (!messages.some((row) => row.id === id)) {
			around = id;
			await load(chat).catch(() => {});
		}
		scroller?.querySelector(`[data-message="${id}"]`)?.scrollIntoView({ block: 'center', behavior: 'smooth' });
		flashing = id;
		setTimeout(() => flashing === id && (flashing = undefined), 1200);
	}
</script>

<div class="chat" bind:this={root}>
	{#if failed}
		<p class="error">Couldn't load chats</p>
	{:else}
		<nav>
			{#each chats as chat (chat.id)}
				<button class="button" class:primary={chat.id === selected} onclick={() => update({ chat: chat.id })}>{chat.name}</button>
			{/each}
		</nav>
		<div class="messages" bind:this={scroller}>
			{#each messages as message (message.id)}
				<Message
					{message}
					flagged={message.flagged}
					flashing={flashing === message.id}
					onflag={() => flag(message)}
					onreply={() => (replying = message)}
					onjump={jump}
				/>
			{/each}
		</div>
		{#if selected !== undefined}
			<Composer onsend={send} {replying} oncancel={() => (replying = null)} {placeholder} --line-start="4px" />
		{/if}
	{/if}
</div>

<style>
	.chat {
		display: flex;
		flex-direction: column;
		height: calc(100dvh - 84px);
	}

	nav {
		display: flex;
		flex: none;
		gap: 8px;
		overflow-x: auto;
	}

	nav button {
		flex: none;
	}

	.messages {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1;
		min-height: 0;
		margin: 0 -8px;
		padding: 12px 0;
		overflow-y: auto;
	}

	.error {
		color: var(--text-muted);
	}
</style>
