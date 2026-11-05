<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { params, update } from '#lib/url.svelte.js';
	import Dialog from '#lib/admin/Dialog.svelte';
	import Composer from '#lib/chat/Composer.svelte';
	import Message, { type ChatMessage } from '#lib/chat/Message.svelte';
	import type { PageProps } from './$types';

	type Author = { id: number; handle: string | null; name: string | null; banned: boolean };
	type Chat = { id: number; name: string; kind: 'all' | 'guild' | 'private'; user: Author | null };
	type Row = ChatMessage & { author: Author; flags: number };
	type Flag = { id: number; chat: { id: number; name: string }; author: Author; contents: string; created_at: string; flags: number };
	type Pending = { title: string; description: string; run: () => Promise<void> };

	const PAGE = 100;

	let { data }: PageProps = $props();

	let chats = $state<Chat[]>([]);
	let selected = $state<number>();
	let messages = $state<Row[]>([]);
	let older = $state(false);
	let flags = $state<Flag[]>([]);
	let flashing = $state<number>();
	let pending = $state<Pending | null>(null);
	let error = $state('');
	let scroller = $state<HTMLElement>();

	const shared = $derived(chats.filter((chat) => chat.kind !== 'private'));
	const banned = $derived(chats.filter((chat) => chat.kind === 'private'));
	const current = $derived(chats.find((chat) => chat.id === selected));
	const wanted = $derived(chats.find((chat) => String(chat.id) === params().get('chat'))?.id ?? chats[0]?.id);
	const focus = $derived(Number(params().get('message')) || undefined);

	async function get<T>(url: string): Promise<T> {
		const response = await fetch(url, { cache: 'no-cache' });
		if (!response.ok) throw new Error(`${response.status}`);
		return response.json();
	}

	async function post(url: string) {
		const response = await fetch(url, { method: 'POST' });
		if (!response.ok) throw new Error((await response.text()) || `${response.status}`);
	}

	async function refresh() {
		[chats, flags] = await Promise.all([get<Chat[]>('/api/admin/moderation/chats'), get<Flag[]>('/api/admin/moderation/flags')]);
	}

	async function open(id: number, from?: number) {
		if (selected !== id) messages = [];
		selected = id;
		try {
			const list = await get<Row[]>(`/api/admin/moderation/chats/${id}/messages${from ? `?from=${from}` : ''}`);
			if (selected !== id) return;
			messages = list;
			older = from ? list.filter((message) => message.id < from).length >= 25 : list.length >= PAGE;
			await tick();
			if (!from) scroller?.scrollTo({ top: scroller.scrollHeight });
		} catch (e) {
			if (selected === id) error = `Couldn't load messages: ${e instanceof Error ? e.message : e}`;
		}
	}

	async function loadOlder() {
		if (!selected || !messages.length) return;
		const id = selected;
		const height = scroller?.scrollHeight ?? 0;
		const list = await get<Row[]>(`/api/admin/moderation/chats/${id}/messages?before=${messages[0].id}`).catch((e) => {
			error = `Couldn't load messages: ${e.message}`;
			return null;
		});
		if (!list || selected !== id) return;
		messages = [...list, ...messages];
		older = list.length >= PAGE;
		await tick();
		scroller?.scrollTo({ top: scroller.scrollHeight - height });
	}

	async function jump(id: number, chat = selected) {
		if (!chat) return;
		if (chat !== selected || !messages.some((message) => message.id === id)) await open(chat, id);
		await tick();
		scroller?.querySelector(`[data-message="${id}"]`)?.scrollIntoView({ block: 'center', behavior: 'smooth' });
		flashing = id;
		setTimeout(() => flashing === id && (flashing = undefined), 1200);
	}

	function choose(chat: number) {
		if (chat !== wanted || focus) update({ chat, message: null });
	}

	function show(id: number, chat: number) {
		if (focus === id && wanted === chat) jump(id, chat);
		else update({ chat, message: id });
	}

	async function act(url: string) {
		error = '';
		try {
			await post(url);
			await refresh();
			if (selected) await open(selected, messages[0]?.id);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function send(contents: string) {
		const id = selected;
		if (!id) return;
		const response = await fetch(`/api/admin/moderation/chats/${id}/messages`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ contents })
		});
		if (!response.ok) {
			error = (await response.text()) || `Couldn't send (${response.status})`;
			throw new Error(error);
		}
		error = '';
		await open(id);
	}

	function remove(id: number, contents: string) {
		pending = { title: `Delete message #${id}`, description: contents, run: () => act(`/api/admin/moderation/messages/${id}/delete`) };
	}

	const who = (author: Author) => author.handle ?? author.name ?? `User #${author.id}`;

	$effect(() => {
		refresh().catch((e) => (error = `Couldn't load moderation: ${e.message}`));
	});

	$effect(() => {
		const chat = wanted;
		const message = focus;
		if (chat === undefined) return;
		untrack(() => (message ? jump(message, chat) : open(chat)));
	});
</script>

<svelte:head>
	<title>Moderation - Hack Club Sunker Admin</title>
</svelte:head>

{#snippet author(author: Author)}
	{#if author.name}<a class="name" href="/admin/users?user={author.id}">{author.name}</a>{/if}
	{#if author.banned}<span class="badge negative">banned</span>{/if}
{/snippet}

<div class="moderation">
	<nav>
		<h2>Chats</h2>
		{#each shared as chat (chat.id)}
			<button class:selected={chat.id === selected} onclick={() => choose(chat.id)}>{chat.name}</button>
		{/each}
		{#if banned.length}
			<h2>Banned users</h2>
			{#each banned as chat (chat.id)}
				<button class:selected={chat.id === selected} onclick={() => choose(chat.id)}>
					{chat.user ? who(chat.user) : chat.name}
				</button>
			{/each}
		{/if}
	</nav>

	<section class="chat">
		<header>
			<h1>{current ? (current.user ? `${who(current.user)} and moderators` : current.name) : 'Moderation'}</h1>
			{#if error}<p class="error">{error}</p>{/if}
		</header>
		<div class="messages" bind:this={scroller}>
			{#if older}<button class="older" onclick={loadOlder}>Load older messages</button>{/if}
			{#each messages as message (message.id)}
				<Message
					{message}
					flagged={message.flags > 0}
					flashing={flashing === message.id}
					onjump={(id) => selected && show(id, selected)}
					ondelete={message.deleted_at ? undefined : () => remove(message.id, message.contents)}
				>
					{#snippet meta()}
						{@render author(message.author)}
						{#if message.flags}<span class="badge flag">{message.flags} {message.flags === 1 ? 'flag' : 'flags'}</span>{/if}
						{#if message.deleted_at}<span class="badge">deleted</span>{/if}
					{/snippet}
				</Message>
			{:else}
				<p class="empty">No messages yet</p>
			{/each}
		</div>
		{#if current?.kind === 'private'}
			{#key selected}<div class="compose"><Composer onsend={send} --line-start="8px" /></div>{/key}
		{/if}
	</section>

	<aside>
		<h2>Flagged ({flags.length})</h2>
		{#each flags as flag (flag.id)}
			<div class="flagged">
				<button class="summary" onclick={() => show(flag.id, flag.chat.id)}>
					<span class="where">{flag.chat.name} · {flag.flags} {flag.flags === 1 ? 'flag' : 'flags'}</span>
					<span class="handle">{who(flag.author)}</span>
					<span class="contents">{flag.contents}</span>
				</button>
				<div class="actions">
					<button onclick={() => act(`/api/admin/moderation/messages/${flag.id}/dismiss`)}>Dismiss</button>
					<button class="danger" onclick={() => remove(flag.id, flag.contents)}>Delete</button>
					{#if data.admin.permissions.ban && !flag.author.banned}
						<button
							class="danger"
							onclick={() =>
								(pending = {
									title: `Ban ${who(flag.author)}`,
									description: 'They lose access to every chat except a private one with the moderators.',
									run: () => act(`/api/admin/moderation/users/${flag.author.id}/ban`)
								})}
						>
							Ban user
						</button>
					{/if}
				</div>
			</div>
		{:else}
			<p class="empty">Nothing flagged</p>
		{/each}
	</aside>
</div>

{#if pending}
	{@const action = pending}
	<Dialog title={action.title} description={action.description} onclose={() => (pending = null)}>
		<div class="confirm">
			<button onclick={() => (pending = null)}>Cancel</button>
			<button
				class="primary danger"
				onclick={async () => {
					const { run } = action;
					pending = null;
					await run();
				}}
			>
				Confirm
			</button>
		</div>
	</Dialog>
{/if}

<style>
	.moderation {
		display: grid;
		grid-template-columns: 200px minmax(0, 1fr) 300px;
		gap: 8px;
		height: calc(100% + 48px);
		margin: -16px -16px -32px;
	}

	nav,
	.chat,
	aside {
		display: flex;
		flex-direction: column;
		min-height: 0;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
	}

	nav,
	aside {
		gap: 4px;
		padding: 16px 12px;
		overflow-y: auto;
	}

	h1 {
		margin: 0;
		font-size: 16px;
		font-weight: 500;
	}

	h2 {
		margin: 8px 0 4px;
		padding: 0 12px;
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	h2:first-child {
		margin-top: 0;
	}

	nav button {
		padding: 8px 12px;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--text-muted);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	nav button:hover {
		background: var(--hover);
		color: var(--text);
	}

	nav button.selected {
		background: var(--hover);
		color: var(--accent);
	}

	.chat header {
		padding: 16px 20px;
		border-bottom: 1px solid var(--island-border);
	}

	.messages {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1;
		padding: 12px 8px;
		overflow-y: auto;
	}

	.compose {
		padding: 0 12px 12px;
	}

	.older {
		align-self: center;
		margin-bottom: 8px;
	}

	.flagged {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-left: 3px solid var(--flag);
		border-radius: 6px;
		background: var(--background);
	}

	.summary {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.summary:hover .contents {
		color: var(--accent);
	}

	.where {
		color: var(--text-muted);
		font-size: 11px;
	}

	.handle {
		font-weight: 500;
	}

	.contents {
		display: -webkit-box;
		overflow: hidden;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow-wrap: anywhere;
	}

	.actions,
	.confirm {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.confirm {
		justify-content: flex-end;
	}

	.actions button,
	.confirm button,
	.older {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.actions button:hover,
	.confirm button:hover,
	.older:hover {
		background: var(--hover);
	}

	button.danger {
		color: var(--negative);
	}

	button.primary.danger {
		border-color: var(--negative);
		background: var(--negative);
		color: var(--background);
	}

	.name {
		color: var(--text-muted);
		font-size: 12px;
		text-decoration: none;
	}

	.name:hover {
		color: var(--accent);
	}

	.badge {
		padding: 1px 6px;
		border: 1px solid var(--island-border);
		border-radius: 4px;
		color: var(--text-muted);
		font-size: 11px;
	}

	.badge.negative {
		border-color: var(--negative);
		color: var(--negative);
	}

	.badge.flag {
		border-color: var(--flag);
		color: var(--flag);
	}

	.empty,
	.error {
		margin: 0;
		padding: 0 12px;
		color: var(--text-muted);
	}

	.error {
		margin-top: 4px;
		padding: 0;
		color: var(--negative);
	}

	@media (max-width: 1100px) {
		.moderation {
			grid-template-columns: 160px minmax(0, 1fr) 240px;
		}
	}

	@media (max-width: 900px), (max-height: 480px) {
		.moderation {
			display: flex;
			flex-direction: column;
			height: auto;
			margin: 0;
		}

		nav {
			flex-direction: row;
			align-items: center;
			padding: 6px 8px;
			overflow-x: auto;
			overflow-y: hidden;
			scrollbar-width: none;
		}

		nav::-webkit-scrollbar {
			display: none;
		}

		nav h2 {
			margin: 0;
			white-space: nowrap;
		}

		nav button {
			flex-shrink: 0;
			white-space: nowrap;
		}

		.chat {
			height: max(240px, 70dvh);
		}
	}

	@media (pointer: coarse) {
		nav button,
		.actions button,
		.confirm button,
		.older {
			min-height: 44px;
		}

		.name {
			padding: 15px 0;
		}
	}
</style>
