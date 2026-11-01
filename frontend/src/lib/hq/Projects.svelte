<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/Dialog.svelte';
	import ProjectCard from '#lib/hq/ProjectCard.svelte';
	import { request } from '#lib/joel/joel.js';
	import { load, projects, type Project } from '#lib/projects.svelte.js';

	let hackatime = $state<{ linked: boolean; projects?: { name: string; seconds: number }[] } | null>(null);
	let doomed = $state<Project | null>(null);
	let confirming = $state(false);
	let error = $state('');

	const taken = $derived(new Map((projects.list ?? []).flatMap((project) => project.hackatime_projects.map((name) => [name, project] as const))));

	onMount(() => {
		load();
		fetch('/api/hackatime/projects', { cache: 'no-cache' })
			.then((response) => (response.ok ? response.json() : Promise.reject()))
			.then((data) => (hackatime = data))
			.catch(() => (hackatime = { linked: false }));
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
</script>

<div class="projects">
	{#if hackatime && !hackatime.linked}
		<div class="link">
			<p>Link Hackatime to progress with your projects!</p>
			<a class="button primary" href="/hackatime/login">Link Hackatime</a>
		</div>
	{/if}

	{#if projects.broken}
		<p class="muted">Couldn't load your projects</p>
	{:else if !projects.list}
		<p class="muted">Loading your projects...</p>
	{:else}
		{#each projects.list as project (project.id)}
			<ProjectCard {project} {hackatime} {taken} ondelete={() => ask(project)} />
		{:else}
			<p class="muted">No projects yet. Claim a contract in the mines to start one.</p>
		{/each}
	{/if}
</div>

<Dialog bind:open={confirming} title="Delete project">
	<p>Delete {doomed?.name ? `"${doomed.name}"` : 'this project'}? This can't be undone.</p>
	{#if error}<p class="error">{error}</p>{/if}
	<footer>
		<button class="button" onclick={() => (confirming = false)}>Cancel</button>
		<button class="button danger" onclick={remove}>Delete</button>
	</footer>
</Dialog>

<style>
	.projects {
		display: flex;
		flex-direction: column;
		gap: 12px;
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
		color: var(--negative);
		font-size: 12px;
	}

	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
