<script lang="ts">
	import { RESOURCE_NAMES } from '#lib/admin/analytics.js';
	import { request } from '#lib/joel/joel.js';
	import { CHIPROCK_PER_HOUR, hours as logged, remaining, ring, type Hackatime, type Project } from '#lib/projects.svelte.js';

	type Field = 'name' | 'description' | 'repo_url';

	let {
		project,
		hackatime,
		taken,
		open = $bindable(false),
		ondelete
	}: { project: Project; hackatime: Hackatime | null; taken: Map<string, Project>; open?: boolean; ondelete: () => void } = $props();

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

	const mark = (index: number, on: boolean) =>
		(project.criteria_ticked = on ? [...project.criteria_ticked.filter((other) => other !== index), index] : project.criteria_ticked.filter((other) => other !== index));

	async function check(index: number, on: boolean) {
		mark(index, on);
		if (!(await send({ criteria_ticked: project.criteria_ticked }))) mark(index, !on);
	}

	const resources = $derived(project.resources ? Object.entries(project.resources) : [['chiprock', Math.floor(logged(project) * CHIPROCK_PER_HOUR)] as const]);
	const left = $derived(project.contract === null ? remaining(project) : null);

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

<article class="project" class:open>
	<button class="head" aria-expanded={open} onclick={() => (open = !open)}>
		{#if project.screenshot_url}<img class="thumb" src={project.screenshot_url} alt="" />{/if}
		<span class="title">
			<span class="name">{project.name || 'Untitled project'}</span>
			<span class="hint">{logged(project).toFixed(1)}h logged</span>
		</span>
		<svg class="toggle" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
	</button>
	{#if open}
		<p class="meta">{project.contract === null ? 'Custom Project' : `Contract Project: ${project.contract}`}</p>
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
		<div class="rewards">
			<div class="field">
				<span class="label">Resources gained on submission</span>
				<ul class="resources">
					{#each resources as [name, amount] (name)}<li>+{amount} {RESOURCE_NAMES[name] ?? name}</li>{/each}
				</ul>
			</div>
			<div class="field">
				<span class="label">Real life prize earned</span>
				<span class="hint">On Shipping, you can claim 1 prize from Ring {ring(project)}</span>
				{#if left !== null}<span class="hint">Log {(Math.ceil(left * 10) / 10).toFixed(1)} more hours for the next Ring!</span>{/if}
			</div>
		</div>
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
		{#if project.criteria?.length}
			<div class="field">
				<span class="label">Contract Criteria</span>
				<ul class="todo">
					{#each project.criteria as criterion, i (i)}
						{@const done = project.criteria_ticked.includes(i)}
						<li>
							<label class:done>
								<input type="checkbox" checked={done} onchange={(event) => check(i, event.currentTarget.checked)} />
								<span>{criterion}</span>
							</label>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if error}<p class="error">{error}</p>{/if}
		<footer>
			<button class="button" onclick={ondelete}>Delete</button>
			<button class="button primary">Ship</button>
		</footer>
	{/if}
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

	.head {
		display: flex;
		align-items: center;
		gap: 12px;
		margin: -16px;
		padding: 12px 16px;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.open .head {
		margin-bottom: 0;
		border-bottom: 1px solid var(--island-border);
		border-radius: 6px 6px 0 0;
	}

	.head:hover,
	.head:focus-visible {
		background: var(--hover);
		outline: none;
	}

	.thumb {
		flex: none;
		width: 72px;
		height: 45px;
		border-radius: 4px;
		object-fit: cover;
	}

	.title {
		display: flex;
		flex: 1;
		align-items: baseline;
		gap: 12px;
		min-width: 0;
	}

	.title .name {
		font: 18px 'Share Tech', sans-serif;
	}

	.title .hint {
		flex: none;
	}

	.toggle {
		flex: none;
		width: 24px;
		height: 24px;
		fill: none;
		stroke: var(--accent);
		stroke-width: 2.5;
		stroke-linecap: round;
		transition: transform 0.2s ease;
	}

	.open .toggle {
		transform: rotate(45deg);
	}

	.meta {
		margin: 0 0 -4px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.todo {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.todo label {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		padding: 4px 0;
		cursor: pointer;
	}

	.todo input {
		margin: 2px 0 0;
		accent-color: var(--accent);
	}

	.todo span {
		transition:
			opacity 0.2s ease,
			color 0.2s ease;
	}

	.todo .done span {
		color: var(--text-muted);
		text-decoration: line-through;
		opacity: 0.6;
	}

	.rewards {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 1px minmax(0, 1fr);
		gap: 12px;
	}

	.rewards::before {
		content: '';
		grid-column: 2;
		grid-row: 1;
		align-self: center;
		height: 70%;
		background: var(--island-border);
	}

	.rewards > :first-child {
		grid-column: 1;
		grid-row: 1;
	}

	.rewards > :last-child {
		grid-column: 3;
		grid-row: 1;
	}

	.resources {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		margin: 4px 0 0;
		padding: 0;
		list-style: none;
	}

	.resources li {
		padding: 2px 6px;
		border-radius: 4px;
		background: var(--background);
		color: var(--positive);
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

	@container (max-width: 400px) {
		.thumb {
			width: 48px;
			height: 30px;
		}

		.title {
			flex-direction: column;
			align-items: stretch;
			gap: 2px;
		}
	}

	@container (max-width: 360px) {
		.top {
			grid-template-columns: minmax(0, 1fr);
		}

		.rewards {
			grid-template-columns: minmax(0, 1fr);
		}

		.rewards::before {
			grid-row: 2;
			grid-column: 1;
			justify-self: center;
			width: 40%;
			height: 1px;
		}

		.rewards > :first-child,
		.rewards > :last-child {
			grid-column: 1;
		}

		.rewards > :last-child {
			grid-row: 3;
		}

		.shot {
			min-height: 160px;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.toggle {
			transition: none;
		}
	}

	@media (pointer: coarse) {
		.head {
			min-height: 44px;
		}

		.hackatime {
			max-height: 264px;
		}

		.hackatime label {
			box-sizing: border-box;
			min-height: 44px;
		}

		.hackatime input,
		.todo input {
			width: 20px;
			height: 20px;
		}

		.todo label {
			box-sizing: border-box;
			min-height: 44px;
			align-items: center;
		}

		.todo input {
			margin: 0;
		}
	}
</style>
