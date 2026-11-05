<script lang="ts">
	import { request } from '#lib/joel/joel.js';
	import type { Project } from '#lib/projects.svelte.js';

	type Hackatime = { linked: boolean; projects?: { name: string; seconds: number }[] };
	type Field = 'name' | 'description' | 'repo_url';

	let {
		project,
		hackatime,
		taken,
		ondelete
	}: { project: Project; hackatime: Hackatime | null; taken: Map<string, Project>; ondelete: () => void } = $props();

	const TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif'];
	const LIMIT = 5 * 1024 * 1024;

	let error = $state('');
	let search = $state('');
	let dragging = $state(false);
	let uploading = $state(false);
	let picker = $state<HTMLInputElement>();
	const pending = new Map<Field, ReturnType<typeof setTimeout>>();

	const owner = (name: string) => {
		const owner = taken.get(name);
		return owner && owner.id !== project.id ? owner : undefined;
	};
	const matches = $derived(
		(hackatime?.projects ?? [])
			.filter(({ name }) => name.toLowerCase().includes(search.trim().toLowerCase()))
			.toSorted((a, b) => Number(!!owner(a.name)) - Number(!!owner(b.name)))
	);

	async function send(changes: Partial<Project>) {
		try {
			await request('PATCH', `/api/projects/${project.id}`, changes);
			error = '';
			return true;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			return false;
		}
	}

	function edit(field: Field) {
		clearTimeout(pending.get(field));
		pending.set(
			field,
			setTimeout(() => commit(field), 600)
		);
	}

	function commit(field: Field) {
		if (!pending.has(field)) return;
		clearTimeout(pending.get(field));
		pending.delete(field);
		send({ [field]: project[field] ?? '' });
	}

	$effect(() => () => [...pending.keys()].forEach(commit));

	const toggle = (name: string, on: boolean) =>
		(project.hackatime_projects = on ? [...project.hackatime_projects.filter((other) => other !== name), name] : project.hackatime_projects.filter((other) => other !== name));

	async function tick(name: string, on: boolean) {
		toggle(name, on);
		if (!(await send({ hackatime_projects: project.hackatime_projects }))) toggle(name, !on);
	}

	async function upload(file: File | null | undefined) {
		if (!file) return;
		if (!TYPES.includes(file.type)) return (error = 'Screenshots must be PNG, JPEG, WebP or GIF');
		if (file.size > LIMIT) return (error = 'Screenshots must be under 5 MB');
		uploading = true;
		try {
			const response = await fetch(`/api/projects/${project.id}/screenshot`, { method: 'PUT', headers: { 'content-type': file.type }, body: file });
			if (!response.ok) throw new Error((await response.text()) || `${response.status}`);
			project.screenshot_url = (await response.json()).screenshot_url;
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		uploading = false;
	}

	const hours = (seconds: number) => `${(seconds / 3600).toFixed(1)}h`;
</script>

<article class="project">
	<p class="meta">Ring {project.ring} contract: {project.contract}</p>
	<div class="top">
		<div class="stack">
			<label class="field">
				<span class="label">Project Name</span>
				<input class="input" type="text" maxlength="100" placeholder="Untitled project" bind:value={project.name} oninput={() => edit('name')} onblur={() => commit('name')} />
			</label>
			<label class="field grow">
				<span class="label">Project Description</span>
				<span class="hint">A brief summary of what your project is and does</span>
				<textarea class="input" maxlength="2000" bind:value={project.description} oninput={() => edit('description')} onblur={() => commit('description')}></textarea>
			</label>
		</div>
		<div class="field">
			<span class="label">Screenshot</span>
			<span class="hint">Screenshot showing your project in action</span>
			<button
				class="shot"
				class:dragging
				disabled={uploading}
				aria-label="Upload a screenshot"
				onclick={() => picker?.click()}
				ondragover={(event) => {
					event.preventDefault();
					dragging = true;
				}}
				ondragleave={() => (dragging = false)}
				ondrop={(event) => {
					event.preventDefault();
					dragging = false;
					upload(event.dataTransfer?.files[0]);
				}}
				onpaste={(event) => {
					const file = [...(event.clipboardData?.files ?? [])].find((file) => file.type.startsWith('image/'));
					if (!file) return;
					event.preventDefault();
					upload(file);
				}}
			>
				{#if project.screenshot_url}
					<img src={project.screenshot_url} alt="" />
				{:else}
					<svg viewBox="0 0 24 24" aria-hidden="true">
						<path d="M12 15V4M7 9l5-5 5 5M4 15v4a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-4" />
					</svg>
				{/if}
			</button>
			<input type="file" accept={TYPES.join(',')} hidden bind:this={picker} onchange={(event) => upload(event.currentTarget.files?.[0])} />
		</div>
	</div>
	<label class="field">
		<span class="label">Code URL</span>
		<span class="hint">Link to wherever the code is hosted (GitHub, GitLab, Bitbucket, etc.)</span>
		<input class="input" type="url" placeholder="https://" maxlength="500" bind:value={project.repo_url} oninput={() => edit('repo_url')} onblur={() => commit('repo_url')} />
	</label>
	<div class="field">
		<span class="label">Hackatime</span>
		{#if !hackatime}
			<span class="hint">Loading Hackatime...</span>
		{:else if !hackatime.linked}
			<span class="hint">Link Hackatime to continue</span>
		{:else}
			<span class="hint">Tick off which Hackatime projects should be attached to this Sunker project</span>
			<input class="input" type="search" placeholder="Search" bind:value={search} />
			<ul class="hackatime">
				{#each matches as { name, seconds } (name)}
					{@const elsewhere = owner(name)}
					<li>
						<label class:elsewhere>
							<input type="checkbox" checked={project.hackatime_projects.includes(name)} disabled={!!elsewhere} onchange={(event) => tick(name, event.currentTarget.checked)} />
							<span class="name">{name}</span>
							<span class="hint">{elsewhere ? `On ${elsewhere.name || 'another project'}` : hours(seconds)}</span>
						</label>
					</li>
				{:else}
					<li class="hint">{hackatime.projects?.length ? 'No matches' : 'No Hackatime projects yet'}</li>
				{/each}
			</ul>
		{/if}
	</div>
	{#if error}<p class="error">{error}</p>{/if}
	<footer>
		<button class="button" onclick={ondelete}>Delete</button>
		<button class="button primary">Ship</button>
	</footer>
</article>

<style>
	.project {
		container-type: inline-size;
		display: flex;
		flex-direction: column;
		gap: 16px;
		padding: 16px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	.meta {
		margin: 0 0 -4px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.top {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: 12px;
	}

	.stack {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
	}

	.grow,
	.grow textarea,
	.shot {
		flex: 1;
	}

	.label {
		font-weight: 500;
	}

	.hint {
		color: var(--text-muted);
		font-size: 12px;
	}

	.field > :is(.input, .shot, .hackatime) {
		margin-top: 4px;
	}

	textarea {
		min-height: 72px;
	}

	.shot {
		display: grid;
		place-items: center;
		min-height: 120px;
		padding: 0;
		overflow: hidden;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text-muted);
		cursor: pointer;
	}

	.shot:hover:not(:disabled),
	.shot.dragging,
	.shot:focus-visible {
		border-color: var(--accent);
		outline: none;
	}

	.shot:disabled {
		cursor: default;
		opacity: 0.5;
	}

	.shot svg {
		width: 24px;
		height: 24px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.shot img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.hackatime {
		max-height: 180px;
		margin: 0;
		padding: 4px;
		overflow: auto;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		list-style: none;
	}

	.hackatime > .hint {
		padding: 6px 8px;
	}

	.hackatime label {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: 4px;
		cursor: pointer;
	}

	.hackatime label:hover {
		background: var(--hover);
	}

	.hackatime label.elsewhere {
		opacity: 0.5;
		cursor: default;
	}

	.hackatime label.elsewhere:hover {
		background: none;
	}

	.hackatime input {
		margin: 0;
		accent-color: var(--accent);
	}

	.name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.error {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
	}

	footer {
		display: flex;
		justify-content: space-between;
	}

	@container (max-width: 360px) {
		.top {
			grid-template-columns: minmax(0, 1fr);
		}

		.shot {
			min-height: 160px;
		}
	}

	@media (pointer: coarse) {
		.hackatime {
			max-height: 264px;
		}

		.hackatime label {
			box-sizing: border-box;
			min-height: 44px;
		}

		.hackatime input {
			width: 20px;
			height: 20px;
		}
	}
</style>
