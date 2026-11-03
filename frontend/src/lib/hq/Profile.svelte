<script lang="ts">
	import Passport from '#lib/onboarding/Passport.svelte';
	import { complete, draft, me, save } from '#lib/me.svelte.js';

	let character = $state(draft(me.current?.character));
	let saving = $state(false);
	let error = $state('');
	let failure = $state('');

	const dirty = $derived(JSON.stringify(character) !== JSON.stringify(draft(me.current?.character)));

	async function submit() {
		saving = true;
		error = failure = '';
		try {
			character = draft(await save(character));
		} catch (e) {
			const text = e instanceof Error ? e.message : String(e);
			if ((e as { status?: number }).status === 409) error = text;
			else failure = text;
		}
		saving = false;
	}
</script>

<div class="profile">
	<Passport bind:draft={character} guild={me.current?.guild} {error} />
	{#if dirty}
		<div class="actions">
			<button class="button" onclick={() => (character = draft(me.current?.character))} disabled={saving}>Discard</button>
			<button class="button primary" onclick={submit} disabled={saving || !complete(character)}>{saving ? 'Saving...' : 'Save'}</button>
		</div>
	{/if}
	{#if failure}<p class="failure">{failure}</p>{/if}
	<a class="button" href="/logout" rel="external">Log out</a>
</div>

<style>
	.profile {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 12px;
	}

	.profile :global(.upper) {
		transform: none;
	}

	.actions {
		display: flex;
		align-self: stretch;
		justify-content: flex-end;
		gap: 8px;
	}

	.failure {
		margin: 0;
		color: var(--negative);
	}
</style>
