<script lang="ts">
	import { untrack } from 'svelte';
	import { fromInput, localInput, relative, request, stamp, type Slot, type Wave } from './joel.js';

	let {
		wave,
		slot,
		now,
		suggest,
		refresh
	}: { wave: Wave; slot: Slot; now: string; suggest: () => { title: string; body: string }; refresh: () => Promise<void> } = $props();

	type Timing = 'draft' | 'at' | 'now' | 'resolve';

	const saved = $derived(wave.announcements[slot]);
	const offset = $derived(new Date(now).getTime() - Date.now());
	const label = $derived(slot === 'warning' ? 'Warning' : 'Follow-up');
	const purpose = $derived(slot === 'warning' ? 'Tells players what is coming before the wave resolves.' : 'Tells players how the wave went.');

	const initial = (): Timing => (!saved ? (slot === 'followup' ? 'resolve' : 'draft') : saved.on_resolve ? 'resolve' : saved.publish_at ? 'at' : 'draft');
	const start = untrack(() => ({ saved, resolve_at: wave.resolve_at, timing: initial() }));

	let open = $state(!!start.saved);
	let title = $state(start.saved?.title ?? '');
	let body = $state(start.saved?.body ?? '');
	let timing = $state<Timing>(start.timing);
	let at = $state(localInput(start.saved?.publish_at ?? start.resolve_at));
	let busy = $state(false);
	let error = $state('');

	const out = $derived(!!saved?.publish_at && new Date(saved.publish_at).getTime() <= Date.now() + offset);
	const status = $derived.by(() => {
		if (!saved) return 'Not saved';
		if (out) return `Published ${stamp(saved.publish_at!)}`;
		if (saved.on_resolve) return 'Publishes when the wave resolves';
		if (saved.publish_at) return `Publishes ${stamp(saved.publish_at)} (${relative(saved.publish_at, Date.now() + offset)})`;
		return 'Draft, not published';
	});

	function prefill() {
		({ title, body } = suggest());
	}

	async function save() {
		busy = true;
		error = '';
		try {
			await request('PUT', `/api/admin/joel/waves/${wave.id}/announcements/${slot}`, {
				title,
				body,
				publish_at: timing === 'now' ? new Date(Date.now() + offset).toISOString() : timing === 'at' ? fromInput(at) : null,
				on_resolve: timing === 'resolve'
			});
			if (timing === 'now') timing = 'at';
			await refresh();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	async function remove() {
		busy = true;
		error = '';
		try {
			if (saved) await request('DELETE', `/api/admin/joel/waves/${wave.id}/announcements/${slot}`);
			open = false;
			title = '';
			body = '';
			timing = initial();
			await refresh();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<div class="card">
	<div class="row spread">
		<strong>{label}</strong>
		{#if open}<span class="muted">{status}</span>{/if}
	</div>
	{#if !open}
		<span class="muted">{purpose}</span>
		<div class="row">
			<button class="button" onclick={() => ((open = true), title || prefill())}>Add {label.toLowerCase()}</button>
		</div>
	{:else}
		<input type="text" bind:value={title} placeholder="Title" maxlength="200" aria-label="{label} title" />
		<textarea rows="5" bind:value={body} placeholder="What players will read" aria-label="{label} body"></textarea>
		<div class="row">
			<label><input type="radio" bind:group={timing} value="draft" />Draft</label>
			<label><input type="radio" bind:group={timing} value="at" />At a time</label>
			<label><input type="radio" bind:group={timing} value="now" />Now</label>
			{#if slot === 'followup'}<label><input type="radio" bind:group={timing} value="resolve" />On resolve</label>{/if}
		</div>
		{#if timing === 'at'}
			<input type="datetime-local" bind:value={at} aria-label="Publish at" />
		{/if}
		{#if error}<span class="error">{error}</span>{/if}
		<div class="row">
			<button class="button primary" disabled={busy || !title.trim() || (timing === 'at' && !at)} onclick={save}>{timing === 'now' ? 'Publish now' : 'Save'}</button>
			<button class="button" disabled={busy} onclick={prefill}>Prefill</button>
			<button class="button danger" disabled={busy} onclick={remove}>{saved ? 'Remove' : 'Discard'}</button>
		</div>
	{/if}
</div>

<style>
	.spread {
		justify-content: space-between;
	}

	label {
		display: flex;
		align-items: center;
		gap: 4px;
		color: var(--text-muted);
		font-size: 12px;
	}

	label input {
		margin: 0;
	}
</style>
