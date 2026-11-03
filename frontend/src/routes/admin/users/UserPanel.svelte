<script lang="ts">
	import Dialog from '#lib/admin/Dialog.svelte';
	import type { Permissions } from '#lib/admin/access.js';
	import Table from './Table.svelte';
	import { ORDER_STATUSES } from '#lib/orders.js';
	import {
		PRIVATE_FIELDS,
		RESOURCES,
		duration,
		format,
		label,
		personName,
		type Detail,
		type Private,
		type Row
	} from './users';

	type Field = {
		key: string;
		label: string;
		type: 'text' | 'number' | 'textarea' | 'select';
		value: string;
		options?: readonly string[];
	};

	type Values = Record<string, string | number | null>;

	type Pending = {
		title: string;
		description?: string;
		kind: string;
		extra?: Row;
		fields?: Field[];
		reasonRequired?: boolean;
		build?: (values: Values) => Row;
	};

	let {
		id,
		permissions,
		onopen,
		onclose,
		onchange
	}: { id: number; permissions: Permissions; onopen: (id: number) => void; onclose: () => void; onchange: () => void } = $props();

	let detail = $state<Detail | null>(null);
	let error = $state('');
	let secrets = $state<Private | null>(null);
	let pending = $state<Pending | null>(null);
	let values = $state<Values>({});
	let reason = $state('');
	let actionError = $state('');
	let busy = $state(false);
	let chats = $state<number[]>([]);
	let guild = $state('');
	let body: HTMLElement;

	const sections = [
		['profile', 'Profile'],
		['access', 'Guild and chats'],
		['character', 'Character'],
		['referrals', 'Referrals'],
		['resources', 'Resources'],
		['projects', 'Projects'],
		['ships', 'Ships'],
		['orders', 'Orders'],
		['messages', 'Messages'],
		['analytics', 'Analytics'],
		['slack', 'Slack'],
		['feedback', 'Feedback'],
		['log', 'Admin log'],
		['activity', 'As admin'],
		['sessions', 'Sessions']
	];

	async function load() {
		const requested = id;
		try {
			const response = await fetch(`/api/admin/users/${requested}`, { cache: 'no-cache' });
			if (requested !== id) return;
			if (!response.ok) {
				error = `Couldn't load user (${response.status})`;
				return;
			}
			const data: Detail = await response.json();
			if (requested !== id) return;
			detail = data;
			chats = data.chats.filter((chat) => chat.allowed).map((chat) => chat.id);
			guild = data.user.guild_id === null ? '' : String(data.user.guild_id);
		} catch {
			if (requested === id) error = "Couldn't load user";
		}
	}

	$effect(() => {
		id;
		detail = null;
		error = '';
		secrets = null;
		pending = null;
		load();
		body?.scrollTo(0, 0);
	});

	async function reveal() {
		const requested = id;
		const response = await fetch(`/api/admin/users/${requested}/private`, { cache: 'no-cache' }).catch(() => null);
		if (response?.ok && requested === id) secrets = await response.json();
	}

	function ask(action: Pending) {
		pending = action;
		values = Object.fromEntries((action.fields ?? []).map((field) => [field.key, field.value]));
		reason = '';
		actionError = '';
	}

	async function confirm() {
		if (!pending) return;
		if (pending.reasonRequired && !reason.trim()) {
			actionError = 'A reason is needed';
			return;
		}
		const fields = Object.fromEntries(
			(pending.fields ?? []).map((field) => {
				const value = values[field.key];
				const parsed = field.type === 'number' ? (value === '' || value === null ? null : Number(value)) : value;
				return [field.key, parsed];
			})
		);
		busy = true;
		const response = await fetch(`/api/admin/users/${id}/action`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ kind: pending.kind, ...pending.extra, ...(pending.build?.(values) ?? fields), reason })
		}).catch(() => null);
		busy = false;
		if (!response) {
			actionError = 'Network error';
			return;
		}
		if (!response.ok) {
			actionError = (await response.text()) || `Failed (${response.status})`;
			return;
		}
		pending = null;
		await load();
		onchange();
	}

	const jump = (section: string) => body.querySelector(`#section-${section}`)?.scrollIntoView({ behavior: 'smooth' });

	const userFields = $derived(detail ? Object.entries(detail.user) : []);
</script>

<aside>
	<header>
		<div class="title">
			<h2>{detail ? personName({ id, name: [detail.user.first_name, detail.user.last_name].filter(Boolean).join(' ') || null }) : `User #${id}`}</h2>
			{#if detail}
				<span class="badge">#{id}</span>
				{#if detail.user.is_banned}<span class="badge negative">banned</span>{/if}
				{#if detail.user.deleted_at}<span class="badge negative">deleted</span>{/if}
			{/if}
		</div>
		<button class="close" aria-label="Close" onclick={onclose}>
			<svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="M2 2L12 12M12 2L2 12" /></svg>
		</button>
	</header>

	{#if detail}
		<nav>
			{#each sections as [key, name] (key)}
				<button onclick={() => jump(key)}>{name}</button>
			{/each}
		</nav>
	{/if}

	<div class="body" bind:this={body}>
		{#if error}
			<p class="error">{error}</p>
		{:else if !detail}
			<p class="muted">Loading...</p>
		{:else}
			{@const d = detail}
			<div class="toolbar">
				{#if permissions.ban}
					{#if d.user.is_banned}
						<button onclick={() => ask({ title: 'Unban user', kind: 'unban' })}>Unban</button>
					{:else}
						<button class="danger" onclick={() => ask({ title: 'Ban user', kind: 'ban', reasonRequired: true })}>Ban</button>
					{/if}
					{#if d.user.deleted_at}
						<button onclick={() => ask({ title: 'Restore user', kind: 'restore' })}>Restore</button>
					{:else}
						<button class="danger" onclick={() => ask({ title: 'Soft delete user', description: 'Sets deleted_at. They stop being able to sign in.', kind: 'delete', reasonRequired: true })}>
							Soft delete
						</button>
					{/if}
				{/if}
				<button
					hidden={!permissions.resources}
					onclick={() =>
						ask({
							title: 'Adjust resources',
							description: 'Positive or negative amounts. Adds a ledger row.',
							kind: 'resources',
							reasonRequired: true,
							fields: RESOURCES.map((name) => ({ key: name, label: label(name), type: 'number', value: '0' })),
							build: (values) => ({ deltas: Object.fromEntries(RESOURCES.map((name) => [name, Number(values[name]) || 0])) })
						})}
				>
					Adjust resources
				</button>
			</div>

			<section id="section-profile">
				<h3>Profile</h3>
				<dl>
					{#each userFields as [key, value] (key)}
						<dt>{label(key)}</dt>
						<dd>{#if value === null}<span class="muted">none</span>{:else}{format(value)}{/if}</dd>
					{/each}
					{#each PRIVATE_FIELDS as key (key)}
						<dt>{label(key)}</dt>
						<dd>
							{#if secrets}
								{#if secrets.user[key] === null || secrets.user[key] === undefined}<span class="muted">none</span>{:else}{format(secrets.user[key])}{/if}
							{:else}
								<button class="masked" onclick={reveal}>&bull;&bull;&bull;&bull;&bull;&bull; reveal</button>
							{/if}
						</dd>
					{/each}
				</dl>
			</section>

			<section id="section-access">
				<h3>Guild and chats</h3>
				<div class="row">
					<select bind:value={guild} disabled={!permissions.chats}>
						<option value="">No guild</option>
						{#each d.guilds as option (option.id)}<option value={String(option.id)}>{option.name}</option>{/each}
					</select>
					<button
						hidden={!permissions.chats}
						disabled={guild === (d.user.guild_id === null ? '' : String(d.user.guild_id))}
						onclick={() =>
							ask({
								title: 'Change guild',
								description: `To ${d.guilds.find((option) => String(option.id) === guild)?.name ?? 'no guild'}`,
								kind: 'guild',
								build: () => ({ guild_id: guild === '' ? null : Number(guild) })
							})}
					>
						Save guild
					</button>
				</div>
				<div class="checks">
					{#each d.chats as chat (chat.id)}
						<label><input type="checkbox" value={chat.id} bind:group={chats} disabled={!permissions.chats} /> {chat.name}</label>
					{/each}
				</div>
				<button
					hidden={!permissions.chats}
					onclick={() =>
						ask({
							title: 'Change allowed chats',
							description: d.chats.filter((chat) => chats.includes(chat.id)).map((chat) => chat.name).join(', ') || 'No chats',
							kind: 'chats',
							build: () => ({ chat_ids: chats })
						})}
				>
					Save chats
				</button>
			</section>

			<section id="section-character">
				<h3>Character</h3>
				{#if d.character}
					<dl>
						{#each Object.entries(d.character) as [key, value] (key)}
							<dt>{label(key)}</dt>
							<dd>{format(value)}</dd>
						{/each}
					</dl>
					<button
						hidden={!permissions.moderation}
						onclick={() =>
							ask({
								title: 'Rename character',
								kind: 'handle',
								fields: [{ key: 'handle', label: 'Handle', type: 'text', value: d.character?.handle ?? '' }]
							})}
					>
						Rename
					</button>
				{:else}
					<p class="muted">No character</p>
				{/if}
			</section>

			<section id="section-referrals">
				<h3>Referrals</h3>
				<p>
					Referred by:
					{#if d.referrer}<button class="link" onclick={() => onopen(d.referrer!.id)}>{personName(d.referrer)}</button>{:else}<span class="muted">nobody</span>{/if}
				</p>
				<p>Referred {d.referrals.length}:</p>
				<div class="people">
					{#each d.referrals as person (person.id)}<button class="link" onclick={() => onopen(person.id)}>{personName(person)}</button>{/each}
				</div>
			</section>

			<section id="section-resources">
				<h3>Resources</h3>
				{#if d.resources}<Table rows={[d.resources]} {onopen} />{/if}
				<h4>Ledger ({d.ledger.length})</h4>
				<Table rows={d.ledger} {onopen} />
			</section>

			<section id="section-projects">
				<h3>Projects ({d.projects.length})</h3>
				<Table rows={d.projects} {onopen}>
					{#snippet actions(row)}
						{#if permissions.moderation}
							{#if row.deleted_at}
								<button onclick={() => ask({ title: `Restore project #${row.id}`, kind: 'project_restore', extra: { project_id: row.id } })}>Restore</button>
							{:else}
								<button class="danger" onclick={() => ask({ title: `Soft delete project #${row.id}`, kind: 'project_delete', extra: { project_id: row.id } })}>Delete</button>
							{/if}
						{/if}
					{/snippet}
				</Table>
			</section>

			<section id="section-ships">
				<h3>Ships ({d.ships.length})</h3>
				<Table rows={d.ships} {onopen}>
					{#snippet actions(row)}
						{#if permissions.moderation}
							<button
								onclick={() =>
									ask({
										title: `Approve ship #${row.id}`,
										description: `Tracked ${duration(Number(row.tracked_seconds))}. No resources are granted automatically.`,
										kind: 'ship_decide',
										extra: { ship_id: row.id, status: 'approved' },
										fields: [
											{ key: 'awarded_ring', label: 'Awarded ring', type: 'number', value: String(row.awarded_ring ?? row.ring) },
											{ key: 'approved_seconds', label: 'Approved seconds', type: 'number', value: String(row.approved_seconds ?? row.tracked_seconds) }
										]
									})}
							>
								Approve
							</button>
							<button
								class="danger"
								onclick={() =>
									ask({
										title: `Reject ship #${row.id}`,
										kind: 'ship_decide',
										extra: { ship_id: row.id, status: 'rejected' },
										fields: [{ key: 'rejection_reason', label: 'Rejection reason (shown to user)', type: 'textarea', value: String(row.rejection_reason ?? '') }]
									})}
							>
								Reject
							</button>
							{#if row.status !== 'pending'}
								<button onclick={() => ask({ title: `Reset ship #${row.id} to pending`, kind: 'ship_decide', extra: { ship_id: row.id, status: 'pending' } })}>Reset</button>
							{/if}
							{#if row.deleted_at}
								<button onclick={() => ask({ title: `Restore ship #${row.id}`, kind: 'ship_restore', extra: { ship_id: row.id } })}>Restore</button>
							{:else}
								<button class="danger" onclick={() => ask({ title: `Soft delete ship #${row.id}`, kind: 'ship_delete', extra: { ship_id: row.id } })}>Delete</button>
							{/if}
						{/if}
					{/snippet}
				</Table>
			</section>

			<section id="section-orders">
				<h3>Orders ({d.orders.length})</h3>
				<Table rows={d.orders.map((order) => ({ ...order, ...(secrets?.orders[String(order.id)] ?? {}) }))} {onopen}>
					{#snippet actions(row)}
						{#if permissions.resources}
							<button
								onclick={() =>
									ask({
										title: `Set order #${row.id} status`,
										kind: 'order_status',
										extra: { order_id: row.id },
										fields: [
											{ key: 'status', label: 'Status', type: 'select', value: String(row.status), options: ORDER_STATUSES },
											{ key: 'tracking_number', label: 'Tracking number', type: 'text', value: String(row.tracking_number ?? '') }
										]
									})}
							>
								Status
							</button>
						{/if}
					{/snippet}
				</Table>
				{#if !secrets && d.orders.length}<button class="masked" onclick={reveal}>Reveal shipping addresses</button>{/if}
			</section>

			<section id="section-messages">
				<h3>Messages ({d.message_count}{d.message_count > d.messages.length ? `, latest ${d.messages.length}` : ''})</h3>
				<Table rows={d.messages} {onopen}>
					{#snippet actions(row)}
						{#if permissions.moderation}
							{#if row.deleted_at}
								<button onclick={() => ask({ title: `Restore message #${row.id}`, kind: 'message_restore', extra: { message_id: row.id } })}>Restore</button>
							{:else}
								<button class="danger" onclick={() => ask({ title: `Delete message #${row.id}`, description: String(row.contents), kind: 'message_delete', extra: { message_id: row.id } })}>Delete</button>
							{/if}
						{/if}
					{/snippet}
				</Table>
			</section>

			<section id="section-analytics">
				<h3>Analytics</h3>
				<div class="stats">
					{#each ['login', 'active'] as kind (kind)}
						<div><span class="muted">{kind}</span> {d.analytics_counts[kind] ?? 0}</div>
					{/each}
				</div>
				<h4>Latest {d.analytics.length} events</h4>
				<Table rows={d.analytics} {onopen} />
			</section>

			<section id="section-slack">
				<h3>Slack</h3>
				{#if d.slack_preference}
					<div class="checks">
						{#each Object.entries(d.slack_preference) as [key, value] (key)}
							<label>
								<input
									type="checkbox"
									checked={value}
									disabled={!permissions.chats}
									onclick={(event) => {
										event.preventDefault();
										ask({ title: `Turn ${value ? 'off' : 'on'} ${label(key)}`, kind: 'slack_preference', extra: { field: key, value: !value } });
									}}
								/>
								{label(key)}
							</label>
						{/each}
					</div>
				{/if}
				<h4>DMs ({d.slack_dms.length})</h4>
				<Table rows={d.slack_dms} {onopen} />
			</section>

			<section id="section-feedback">
				<h3>Feedback ({d.feedback.length})</h3>
				<Table rows={d.feedback} {onopen} />
			</section>

			<section id="section-log">
				<h3>Admin actions on this user ({d.admin_actions.length})</h3>
				<Table rows={d.admin_actions} {onopen} />
			</section>

			<section id="section-activity">
				<h3>As admin</h3>
				<h4>Actions taken ({d.admin_actions_by.length})</h4>
				<Table rows={d.admin_actions_by} {onopen} />
				<h4>Ledger entries made ({d.ledger_by.length})</h4>
				<Table rows={d.ledger_by} {onopen} />
				<h4>Ships reviewed ({d.reviewed_ships.length})</h4>
				<Table rows={d.reviewed_ships} {onopen} />
			</section>

			<section id="section-sessions">
				<h3>Sessions and tokens</h3>
				<Table rows={d.sessions} {onopen} />
				<h4>Linked tokens</h4>
				<Table rows={d.auth_tokens} {onopen} />
			</section>
		{/if}
	</div>
</aside>

{#if pending}
	<Dialog title={pending.title} description={pending.description} onclose={() => (pending = null)}>
		<form
			onsubmit={(event) => {
				event.preventDefault();
				confirm();
			}}
		>
			<div class="fields" class:grid={(pending.fields?.length ?? 0) > 3}>
				{#each pending.fields ?? [] as field (field.key)}
					<label>
						<span>{field.label}</span>
						{#if field.type === 'textarea'}
							<textarea bind:value={values[field.key]} rows="3"></textarea>
						{:else if field.type === 'select'}
							<select bind:value={values[field.key]}>
								{#each field.options ?? [] as option (option)}<option value={option}>{option}</option>{/each}
							</select>
						{:else}
							<input type={field.type} bind:value={values[field.key]} />
						{/if}
					</label>
				{/each}
			</div>
			<label>
				<span>Reason{pending.reasonRequired ? '' : ' (optional)'}, logged to admin actions</span>
				<textarea bind:value={reason} rows="2"></textarea>
			</label>
			{#if actionError}<p class="error">{actionError}</p>{/if}
			<div class="row end">
				<button type="button" onclick={() => (pending = null)}>Cancel</button>
				<button type="submit" class="primary" disabled={busy}>Confirm</button>
			</div>
		</form>
	</Dialog>
{/if}

<style>
	aside {
		display: flex;
		flex-direction: column;
		min-width: 0;
		height: 100%;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		overflow: hidden;
	}

	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 12px;
		padding: 14px 16px;
		border-bottom: 1px solid var(--island-border);
	}

	.title {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	h2 {
		margin: 0;
		font-size: 17px;
		font-weight: 500;
	}

	h3 {
		margin: 0 0 10px;
		font-size: 14px;
		font-weight: 500;
	}

	h4 {
		margin: 14px 0 8px;
		color: var(--text-muted);
		font-size: 12px;
		font-weight: 500;
	}

	nav {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--island-border);
	}

	nav button {
		padding: 3px 8px;
		border: none;
		background: none;
		color: var(--text-muted);
		font-size: 12px;
	}

	nav button:hover {
		background: var(--hover);
		color: var(--text);
	}

	.body {
		flex: 1;
		padding: 16px;
		overflow: auto;
	}

	section {
		padding: 16px 0;
		border-top: 1px solid var(--island-border);
	}

	.toolbar {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding-bottom: 16px;
	}

	.toolbar + section {
		border-top: 1px solid var(--island-border);
	}

	dl {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 4px 16px;
		margin: 0 0 12px;
		font-size: 13px;
	}

	dt {
		color: var(--text-muted);
		text-transform: capitalize;
	}

	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}

	p {
		margin: 0 0 8px;
		font-size: 13px;
	}

	.muted {
		color: var(--text-muted);
	}

	.error {
		color: var(--negative);
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

	button {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	button:hover:not(:disabled) {
		background: var(--hover);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	button.danger {
		color: var(--negative);
	}

	button.primary {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--background);
	}

	button.primary:hover:not(:disabled) {
		background: var(--accent);
		filter: brightness(1.1);
	}

	.close {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: none;
		background: none;
	}

	.close path {
		stroke: var(--text-muted);
		stroke-width: 1.5;
		stroke-linecap: round;
	}

	.link,
	.masked {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent);
		font-size: 13px;
	}

	.link:hover,
	.masked:hover {
		background: none !important;
		text-decoration: underline;
	}

	.people {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 12px;
	}

	.row {
		display: flex;
		gap: 8px;
		margin-bottom: 12px;
	}

	.row.end {
		justify-content: flex-end;
		margin: 0;
	}

	.checks {
		display: flex;
		flex-wrap: wrap;
		gap: 6px 16px;
		margin-bottom: 12px;
		font-size: 13px;
	}

	.checks label {
		display: flex;
		align-items: center;
		gap: 6px;
		text-transform: capitalize;
	}

	.stats {
		display: flex;
		gap: 24px;
		font-size: 13px;
	}

	select,
	input:not([type='checkbox']),
	textarea {
		padding: 6px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.fields {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.fields.grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}

	form label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 12px;
		color: var(--text-muted);
	}

	textarea {
		resize: vertical;
	}
</style>
