<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/Dialog.svelte';
	import ContractPicker from '#lib/hq/ContractPicker.svelte';
	import ProjectCard from '#lib/hq/ProjectCard.svelte';
	import { request } from '#lib/joel/joel.js';
	import { browse, catalog } from '#lib/contracts.svelte.js';
	import { link, load, projects, type Project } from '#lib/projects.svelte.js';

	let doomed = $state<Project | null>(null);
	let confirming = $state(false);
	let error = $state('');
	let choosing = $state(false);
	let picking = $state(false);
	let creating = $state(false);
	let failure = $state('');
	let corner = $state<HTMLElement>();
	let fresh = $state<number | null>(null);

	const hackatime = $derived(projects.hackatime);
	const taken = $derived(new Map((projects.list ?? []).flatMap((project) => project.hackatime_projects.map((name) => [name, project] as const))));

	onMount(() => {
		load();
		link();
		if (!catalog.loaded) browse();
	});

	function ask(project: Project) {
		doomed = project;
		error = '';
		confirming = true;
	}

	async function remove() {
		if (!doomed) return;
		try {
			await request('DELETE', `/api/projects/${doomed.id}`);
			confirming = false;
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function create(contract: string | null) {
		creating = true;
		try {
			fresh = (await request('POST', '/api/projects', contract ? { contract } : {})).id;
			failure = '';
			choosing = picking = false;
			await load();
		} catch (e) {
			failure = e instanceof Error ? e.message : String(e);
		}
		creating = false;
	}

	function dismiss(event: PointerEvent) {
		if (choosing && !corner?.contains(event.target as Node)) choosing = false;
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape' || !choosing) return;
		event.stopPropagation();
		choosing = false;
	}
</script>

<svelte:window onpointerdown={dismiss} />

<div class="projects">
	{#if hackatime && !hackatime.linked}
		<div class="link">
			<p>Link Hackatime to be able to create projects!</p>
			<a class="button primary" href="/hackatime/login">Link Hackatime</a>
		</div>
	{/if}

	{#if projects.broken}
		<p class="muted">Couldn't load your projects</p>
	{:else if !projects.list}
		<p class="muted">Loading your projects...</p>
	{:else}
		{#each projects.list as project (project.id)}
			<ProjectCard {project} {hackatime} {taken} open={project.id === fresh} ondelete={() => ask(project)} />
		{:else}
			{#if hackatime?.linked}
				<p class="muted">Create your first project with the button below!</p>
			{/if}
		{/each}
	{/if}

	<div class="corner" bind:this={corner}>
		{#if choosing}
			<div class="kinds" role="menu" tabindex="-1" {onkeydown}>
				<button class="kind" role="menuitem" disabled={creating} onclick={() => ((choosing = false), (picking = true))}>
					<strong>Contract</strong>
					<span>Tackle a specific project that Sunker Command needs. These get you the more exotic resources.</span>
				</button>
				<button class="kind" role="menuitem" disabled={creating} onclick={() => create(null)}>
					<strong>Custom</strong>
					<span>Base a project on something that's been in your head for a while to get the Chiprock needed to get operations moving!</span>
				</button>
				{#if failure}<p class="error">{failure}</p>{/if}
			</div>
		{/if}
		<button class="add" aria-label="New project" aria-expanded={choosing} disabled={!hackatime?.linked} {onkeydown} onclick={() => ((choosing = !choosing), (failure = ''))}>
			<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
		</button>
	</div>
</div>

<Dialog bind:open={confirming} title="Delete project">
	<p>Delete {doomed?.name ? `"${doomed.name}"` : 'this project'}? This can't be undone.</p>
	{#if error}<p class="error">{error}</p>{/if}
	<footer>
		<button class="button" onclick={() => (confirming = false)}>Cancel</button>
		<button class="button danger" onclick={remove}>Delete</button>
	</footer>
</Dialog>

<Dialog bind:open={picking} title="Pick a contract" wide>
	<ContractPicker busy={creating} {failure} onstart={create} />
</Dialog>

<style>
	.projects {
		container-type: inline-size;
		display: flex;
		flex-direction: column;
		gap: 12px;
		min-height: calc(100dvh - 84px);
	}

	.link {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	.link p {
		flex: 1;
		margin: 0;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
	}

	p {
		margin: 0 0 16px;
	}

	.error {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
	}

	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	.corner {
		position: sticky;
		bottom: 16px;
		z-index: 2;
		align-self: flex-end;
		margin-top: auto;
	}

	.add {
		display: grid;
		place-items: center;
		width: 64px;
		height: 64px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: var(--accent);
		color: var(--background);
		box-shadow: 0 6px 20px var(--shadow);
		cursor: pointer;
	}

	.add:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.add:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.add svg {
		width: 32px;
		height: 32px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2.5;
		stroke-linecap: round;
	}

	.kinds {
		position: absolute;
		right: 0;
		bottom: calc(100% + 12px);
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
		box-sizing: border-box;
		width: 100cqw;
		padding: 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		box-shadow: 0 16px 48px var(--shadow);
	}

	.kind {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 16px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: 13px/1.5 'Inter', sans-serif;
		text-align: left;
		cursor: pointer;
	}

	.kind:hover:not(:disabled),
	.kind:focus-visible {
		border-color: var(--accent);
		outline: none;
	}

	.kind:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.kind strong {
		color: var(--accent);
		font: 22px 'Share Tech', sans-serif;
	}

	.kind span {
		color: var(--text-muted);
	}

	.kinds .error {
		grid-column: 1 / -1;
	}

	@container (max-width: 360px) {
		.kinds {
			grid-template-columns: 1fr;
		}
	}

	@media (pointer: coarse), (orientation: portrait) and (max-width: 600px) {
		.projects {
			flex: 1;
			min-height: auto;
		}
	}
</style>
