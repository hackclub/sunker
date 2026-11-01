<script lang="ts">
	import { update } from '#lib/url.svelte.js';
	import { STATUSES, relative, stamp, type Announcement, type Wave } from './joel.js';

	let { wave, now, latest }: { wave: Wave; now: string; latest: number | null } = $props();

	const offset = $derived(new Date(now).getTime() - Date.now());

	function announced(announcement?: Announcement) {
		if (!announcement) return null;
		if (announcement.on_resolve) return wave.status === 'resolved' ? 'out' : 'on resolve';
		if (!announcement.publish_at) return 'draft';
		return new Date(announcement.publish_at).getTime() <= Date.now() + offset ? 'out' : relative(announcement.publish_at, Date.now() + offset);
	}

	const warning = $derived(announced(wave.announcements.warning));
	const followup = $derived(announced(wave.announcements.followup));
	const when = $derived(
		wave.status === 'scheduled' && wave.resolve_at
			? `Resolves ${relative(wave.resolve_at, Date.now() + offset)}`
			: wave.resolved_at
				? `Resolved ${stamp(wave.resolved_at)}`
				: `Edited ${stamp(wave.updated_at)}`
	);
</script>

<button class="wave" onclick={() => update({ wave: wave.id, tile: null })}>
	<span class="top">
		<strong>{wave.name}</strong>
		<span class="badge {wave.status}">{STATUSES[wave.status]}</span>
	</span>
	<span class="muted">{wave.orders.length} {wave.orders.length === 1 ? 'order' : 'orders'} - {when}</span>
	<span class="top">
		{#if !warning && !followup}<span class="badge">Silent</span>{/if}
		{#if warning}<span class="badge">Warning: {warning}</span>{/if}
		{#if followup}<span class="badge">Follow-up: {followup}</span>{/if}
		{#if wave.id === latest}<span class="badge">Latest</span>{/if}
	</span>
</button>

<style>
	.wave {
		display: flex;
		flex-direction: column;
		gap: 6px;
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.wave:hover {
		border-color: var(--accent);
	}

	.top {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
	}

	.top:last-child {
		justify-content: flex-start;
	}
</style>
