<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '#lib/admin/Dialog.svelte';
	import Segmented from '#lib/admin/Segmented.svelte';
	import { COLOURS, DEFAULTS } from '#lib/colors.js';
	import { request } from '#lib/joel/joel.js';
	import { refresh as restyle } from '#lib/styles.svelte.js';

	type Person = { id: number; name: string | null } | null;
	type Version = { id: number; width: number; height: number; content_type: string; bytes: number; original: boolean; active: boolean; created_at: string; uploaded_by: Person };
	type Slot = { slug: string; name: string; brief: string; width: number | null; height: number | null; built_in: boolean; created_at: string; versions: Version[] };
	type Change = { id: number; value: string; active: boolean; created_at: string; set_by: Person };
	type Styling = { slots: Slot[]; colours: Record<string, Change[]>; progress: Record<string, { at: string; by: Person }> };
	type Filter = 'all' | 'todo' | 'done';

	const FILTERS: { value: Filter; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'todo', label: 'To do' },
		{ value: 'done', label: 'Completed' }
	];

	let styling = $state<Styling | null>(null);
	let failure = $state('');
	let filter = $state<Filter>('all');
	let search = $state('');
	let find = $state('');
	let opened = $state<string | null>(null);
	let declaring = $state<{ name: string; count: number; brief: string; width: number | null; height: number | null } | null>(null);
	let edit = $state({ name: '', brief: '', width: null as number | null, height: null as number | null });
	let pending = $state<{ file: File; width: number; height: number } | null>(null);
	let confirming = $state<string | null>(null);
	let history = $state<string | null>(null);
	let busy = $state(false);
	let error = $state('');
	let scope = $state<'images' | 'colours'>('images');

	const slot = $derived(styling?.slots.find((s) => s.slug === opened) ?? null);
	const active = (s: Slot) => s.versions.find((v) => v.active) ?? null;
	const expected = (s: Slot) => {
		const original = s.versions.find((v) => v.original);
		return original ? [original.width, original.height] : s.width && s.height ? [s.width, s.height] : null;
	};
	const done = (slug: string) => Boolean(styling?.progress[slug]);
	const matches = (text: string, query: string) => text.toLowerCase().includes(query.trim().toLowerCase());
	const display = (person: Person) => (person ? (person.name ?? `#${person.id}`) : 'the original files');
	const when = (stamp: string) => new Date(stamp).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
	const kilobytes = (bytes: number) => `${(bytes / 1024).toFixed(1)} KB`;
	const label = (key: string) => key.charAt(0).toUpperCase() + key.slice(1).replaceAll('-', ' ');
	const current = (key: string) => styling?.colours[key]?.find((c) => c.active)?.value ?? DEFAULTS[key];

	const images = $derived(styling?.slots.filter((s) => (filter === 'all' || done(s.slug) === (filter === 'done')) && matches(`${s.name} ${s.slug}`, search)) ?? []);
	const groups = $derived(
		Object.entries(COLOURS)
			.map(([group, keys]) => [group, Object.keys(keys).filter((key) => matches(`${label(key)} ${key}`, find))] as const)
			.filter(([, keys]) => keys.length)
	);
	const completed = $derived(styling?.slots.filter((s) => done(s.slug)).length ?? 0);

	async function load() {
		try {
			styling = await request('GET', '/api/admin/styling');
			failure = '';
		} catch (e) {
			failure = `Couldn't load styling: ${e instanceof Error ? e.message : e}`;
		}
	}

	onMount(load);

	async function act(work: () => Promise<unknown>, place: typeof scope = 'images') {
		busy = true;
		error = '';
		scope = place;
		try {
			await work();
			await Promise.all([load(), restyle()]);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		busy = false;
	}

	function open(s: Slot) {
		opened = s.slug;
		edit = { name: s.name, brief: s.brief, width: s.width, height: s.height };
		pending = null;
		confirming = null;
		error = '';
	}

	async function upload(file: File) {
		const response = await fetch(`/api/admin/styling/images/${opened}`, { method: 'POST', headers: { 'content-type': file.type }, body: file });
		if (!response.ok) throw new Error((await response.text()) || `${response.status}`);
		pending = null;
	}

	async function choose(file: File | undefined) {
		if (!file || !slot) return;
		error = '';
		const url = URL.createObjectURL(file);
		const picture = new Image();
		picture.src = url;
		try {
			await picture.decode();
		} catch {
			error = "That file couldn't be read as an image";
			return;
		} finally {
			URL.revokeObjectURL(url);
		}
		const size = expected(slot);
		if (size && (picture.naturalWidth !== size[0] || picture.naturalHeight !== size[1])) pending = { file, width: picture.naturalWidth, height: picture.naturalHeight };
		else act(() => upload(file));
	}

	function confirm(id: string, work: () => Promise<unknown>) {
		if (confirming !== id) {
			confirming = id;
			return;
		}
		confirming = null;
		act(work);
	}

	const side = (value: number | null) => (value && value > 0 ? Math.round(value) : null);
	const setColour = (key: string, value: string | null) => act(() => request('PUT', `/api/admin/styling/colours/${key}`, { value }), 'colours');
	const mark = (slug: string) => act(() => request('PUT', '/api/admin/styling/progress', { key: slug, completed: !done(slug) }));

	function typed(key: string, input: HTMLInputElement) {
		const value = input.value.trim().toLowerCase();
		const alpha = DEFAULTS[key].length === 9;
		if (alpha ? /^#[0-9a-f]{6}([0-9a-f]{2})?$/.test(value) : /^#[0-9a-f]{6}$/.test(value)) {
			if (value !== current(key)) setColour(key, value);
		} else {
			scope = 'colours';
			error = `${label(key)}: colours look like #rrggbb${alpha ? ' or #rrggbbaa' : ''}`;
			input.value = current(key);
		}
	}
</script>

<svelte:head>
	<title>Styling - Hack Club Sunker Admin</title>
</svelte:head>

<div class="styling">
	<div class="images">
		<header>
			<h1>Styling</h1>
			<Segmented options={FILTERS} value={filter} label="Filter" onchange={(value) => (filter = value)} />
			<input class="search" type="search" placeholder="Search images" bind:value={search} />
			<button class="button primary" onclick={() => (declaring = { name: '', count: 1, brief: '', width: null, height: null })}>Declare image</button>
		</header>

		{#if !styling}
			<p class="muted">{failure || 'Loading styling...'}</p>
		{:else}
			<div class="progress">
				<span>{completed} of {styling.slots.length} images completed</span>
				<div class="bar"><div style:width="{styling.slots.length ? (completed / styling.slots.length) * 100 : 0}%"></div></div>
			</div>
			{#if error && scope === 'images' && !opened && !declaring}<p class="error">{error}</p>{/if}
			<div class="grid">
				{#each images as s (s.slug)}
					{@const version = active(s)}
					<button class="card" onclick={() => open(s)}>
						<div class="preview">
							{#if version}
								<img src="/api/admin/styling/versions/{version.id}" alt="" />
							{:else}
								<span class="muted">Not drawn yet</span>
							{/if}
						</div>
						<div class="meta">
							<strong>{s.name}</strong>
							{#if done(s.slug)}<span class="done">Completed</span>{/if}
						</div>
						<span class="muted">
							{version ? `${version.width}x${version.height}` : s.width && s.height ? `${s.width}x${s.height} suggested` : 'No size yet'},
							{s.versions.length} {s.versions.length === 1 ? 'version' : 'versions'}{s.built_in ? '' : ', declared'}
						</span>
					</button>
				{:else}
					<p class="muted">No images match</p>
				{/each}
			</div>
		{/if}
	</div>

	<aside class="palette">
		<header>
			<h2>Colours</h2>
			<input class="find" type="search" placeholder="Search colours" bind:value={find} />
			{#if error && scope === 'colours'}<p class="error wide">{error}</p>{/if}
		</header>
		{#if styling}
			{#each groups as [group, keys] (group)}
				<details open>
					<summary>{group}</summary>
					{#each keys as key (key)}
						{@const value = current(key)}
						{@const changes = styling.colours[key] ?? []}
						{@const changed = changes.some((c) => c.active)}
						<div class="colour" class:changed>
							<input class="swatch" type="color" value={value.slice(0, 7)} aria-label="Pick {label(key)}" disabled={busy} onchange={(event) => setColour(key, event.currentTarget.value + value.slice(7))} />
							<span class="name" title={key}>{label(key)}</span>
							<input class="hex" type="text" {value} aria-label="{label(key)} hex" disabled={busy} onchange={(event) => typed(key, event.currentTarget)} />
							<button class="icon" class:hidden={!changed} title="Reset to default" aria-label="Reset {label(key)} to default" disabled={busy || !changed} onclick={() => setColour(key, null)}>
								<svg viewBox="0 0 16 16"><path d="M3 8a5 5 0 1 0 1.5-3.5M3 2.5v2.5h2.5" /></svg>
							</button>
							<button class="icon" class:hidden={!changes.length} class:open={history === key} title="History" aria-label="{label(key)} history" disabled={!changes.length} onclick={() => (history = history === key ? null : key)}>
								<svg viewBox="0 0 16 16"><path d="M8 4.5V8l2.5 1.5M14 8A6 6 0 1 1 2 8a6 6 0 0 1 12 0z" /></svg>
							</button>
						</div>
						{#if history === key}
							<ul class="history">
								<li>
									<span class="chip" style:background={DEFAULTS[key]}></span>
									<code>{DEFAULTS[key]}</code>
									<span class="muted grow">Default</span>
									{#if changed}<button class="link" disabled={busy} onclick={() => setColour(key, null)}>Use</button>{/if}
								</li>
								{#each changes as change (change.id)}
									<li title="Set by {display(change.set_by)} {when(change.created_at)}">
										<span class="chip" style:background={change.value}></span>
										<code>{change.value}</code>
										<span class="muted grow">{change.active ? 'In use' : when(change.created_at)}</span>
										{#if !change.active}<button class="link" disabled={busy} onclick={() => setColour(key, change.value)}>Use</button>{/if}
									</li>
								{/each}
							</ul>
						{/if}
					{/each}
				</details>
			{:else}
				<p class="muted">No colours match</p>
			{/each}
		{/if}
	</aside>
</div>

{#if slot}
	{@const version = active(slot)}
	{@const size = expected(slot)}
	<Dialog title={slot.name} description="{slot.slug}{slot.built_in ? '' : ', declared'}" onclose={() => (opened = null)}>
		<div class="detail">
			<div class="stage">
				{#if version}
					<img src="/api/admin/styling/versions/{version.id}" alt={slot.name} />
				{:else}
					<span class="muted">Not drawn yet</span>
				{/if}
			</div>
			<div class="side">
				{#if slot.brief}<p class="brief">{slot.brief}</p>{/if}
				<p class="muted">
					{version ? `In use: ${version.width}x${version.height}, ${kilobytes(version.bytes)}` : 'Nothing uploaded yet'}{size ? `. Expected ${size[0]}x${size[1]}` : ''}
				</p>
				<label class="button primary upload" class:disabled={busy}>
					Upload new version
					<input type="file" accept="image/png,image/webp,image/gif" disabled={busy} onchange={(event) => (choose(event.currentTarget.files?.[0]), (event.currentTarget.value = ''))} />
				</label>
				{#if pending && size}
					<div class="warning">
						<p>This image is {pending.width}x{pending.height}, but {slot.name} is expected to be {size[0]}x{size[1]}. The game scales it to fit, so it may look stretched or blurry.</p>
						<div class="row">
							<button class="button" onclick={() => (pending = null)}>Cancel</button>
							<button class="button primary" disabled={busy} onclick={() => act(() => upload(pending!.file))}>Upload anyway</button>
						</div>
					</div>
				{/if}
				<label class="check"><input type="checkbox" checked={done(slot.slug)} disabled={busy} onchange={() => mark(slot.slug)} />Completed</label>
				{#if error}<p class="error">{error}</p>{/if}
			</div>
		</div>

		<h3>Versions</h3>
		<ul class="versions">
			{#each slot.versions as v (v.id)}
				<li class:current={v.active}>
					<img src="/api/admin/styling/versions/{v.id}" alt="" />
					<div class="grow">
						<strong>{v.original ? 'Original' : `Version #${v.id}`}</strong>
						{#if v.active}<span class="done">In use</span>{/if}
						<p class="muted">{v.width}x{v.height}, {kilobytes(v.bytes)}, {v.content_type.replace('image/', '').toUpperCase()}, from {display(v.uploaded_by)} {when(v.created_at)}</p>
					</div>
					{#if !v.active}
						<button class="button" disabled={busy} onclick={() => act(() => request('POST', `/api/admin/styling/versions/${v.id}/activate`))}>Use this</button>
					{/if}
					{#if !v.active && !v.original}
						<button class="button danger" class:primary={confirming === `version-${v.id}`} disabled={busy} onclick={() => confirm(`version-${v.id}`, () => request('DELETE', `/api/admin/styling/versions/${v.id}`))}>
							{confirming === `version-${v.id}` ? 'Confirm delete' : 'Delete'}
						</button>
					{/if}
				</li>
			{:else}
				<li class="muted">No versions yet</li>
			{/each}
		</ul>

		<h3>Brief</h3>
		<form
			onsubmit={(event) => {
				event.preventDefault();
				act(() => request('PUT', `/api/admin/styling/slots/${slot.slug}`, { ...edit, width: side(edit.width), height: side(edit.height) }));
			}}
		>
			<label>Name<input type="text" bind:value={edit.name} maxlength="60" required /></label>
			<label>Brief for the artist<textarea rows="3" bind:value={edit.brief} maxlength="2000"></textarea></label>
			<div class="pair">
				<label>Suggested width<input type="number" min="1" max="8192" bind:value={edit.width} /></label>
				<label>Suggested height<input type="number" min="1" max="8192" bind:value={edit.height} /></label>
			</div>
			<footer>
				{#if !slot.built_in}
					<button
						type="button"
						class="button danger"
						class:primary={confirming === 'slot'}
						disabled={busy}
						onclick={() =>
							confirm('slot', async () => {
								await request('DELETE', `/api/admin/styling/slots/${slot.slug}`);
								opened = null;
							})}
					>
						{confirming === 'slot' ? 'Confirm remove, deleting every version' : 'Remove image'}
					</button>
				{/if}
				<span class="grow"></span>
				<button type="submit" class="button primary" disabled={busy}>Save brief</button>
			</footer>
		</form>
	</Dialog>
{/if}

{#if declaring}
	<Dialog title="Declare image" description="Add images that should be drawn. New ones are numbered after any existing images with the same name." onclose={() => (declaring = null)}>
		<form
			onsubmit={(event) => {
				event.preventDefault();
				const body = { ...declaring!, width: side(declaring!.width), height: side(declaring!.height) };
				act(async () => {
					await request('POST', '/api/admin/styling/slots', body);
					declaring = null;
				});
			}}
		>
			<div class="pair">
				<label>Name<input type="text" bind:value={declaring.name} maxlength="60" placeholder="Building" required /></label>
				<label>How many<input type="number" min="1" max="50" bind:value={declaring.count} required /></label>
			</div>
			<label>Brief for the artist<textarea rows="3" bind:value={declaring.brief} maxlength="2000"></textarea></label>
			<div class="pair">
				<label>Suggested width<input type="number" min="1" max="8192" bind:value={declaring.width} /></label>
				<label>Suggested height<input type="number" min="1" max="8192" bind:value={declaring.height} /></label>
			</div>
			{#if error}<p class="error">{error}</p>{/if}
			<footer>
				<span class="grow"></span>
				<button type="button" class="button" onclick={() => (declaring = null)}>Cancel</button>
				<button type="submit" class="button primary" disabled={busy}>Declare</button>
			</footer>
		</form>
	</Dialog>
{/if}

<style>
	.styling {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 280px;
		align-items: start;
		gap: 16px;
	}

	.images {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.palette {
		position: sticky;
		top: 0;
		display: flex;
		flex-direction: column;
		max-height: calc(100dvh - 64px);
		overflow: auto;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
	}

	.palette header {
		position: sticky;
		top: 0;
		z-index: 1;
		padding: 10px;
		border-bottom: 1px solid var(--island-border);
		background: var(--island);
	}

	.find {
		flex: 1;
		min-width: 0;
		padding: 4px 8px;
		font-size: 12px;
	}

	summary {
		padding: 8px 10px 4px;
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		cursor: pointer;
	}

	details + details {
		border-top: 1px solid var(--island-border);
	}

	details[open] {
		padding-bottom: 6px;
	}

	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}

	h1 {
		margin: 0 auto 0 0;
		font-size: 20px;
		font-weight: 500;
	}

	h2,
	h3 {
		margin: 0;
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	h3 {
		margin: 20px 0 8px;
	}

	.progress {
		display: flex;
		align-items: center;
		gap: 12px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.bar {
		flex: 1;
		height: 6px;
		overflow: hidden;
		border-radius: 3px;
		background: var(--island);
	}

	.bar div {
		height: 100%;
		background: var(--positive);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 8px;
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
		text-align: left;
		cursor: pointer;
	}

	.card:hover {
		background: var(--hover);
	}

	.preview,
	.stage {
		display: grid;
		place-items: center;
		border-radius: 8px;
		background: repeating-conic-gradient(var(--island-border) 0 25%, var(--background) 0 50%) 0 0 / 16px 16px;
	}

	.preview {
		box-sizing: border-box;
		grid-template: minmax(0, 1fr) / minmax(0, 1fr);
		height: 120px;
		padding: 8px;
	}

	.preview img,
	.stage img {
		max-width: 100%;
		max-height: 100%;
		image-rendering: pixelated;
	}

	.preview img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.meta {
		display: flex;
		justify-content: space-between;
		gap: 8px;
	}

	.done {
		color: var(--positive);
		font-size: 12px;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
		font-size: 12px;
	}

	.colour {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 2px 10px;
	}

	.colour:hover {
		background: var(--hover);
	}

	.swatch {
		flex: none;
		width: 20px;
		height: 20px;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 4px;
		background: none;
		cursor: pointer;
	}

	.swatch::-webkit-color-swatch-wrapper {
		padding: 0;
	}

	.swatch::-webkit-color-swatch {
		border: none;
		border-radius: 3px;
	}

	.swatch::-moz-color-swatch {
		border: none;
		border-radius: 3px;
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		font-size: 12px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.changed .name {
		color: var(--accent);
	}

	.hex {
		flex: none;
		width: 78px;
		padding: 2px 4px;
		border-color: transparent;
		background: none;
		font-family: 'JetBrains Mono', monospace;
		font-size: 11px;
	}

	.hex:hover,
	.hex:focus {
		border-color: var(--island-border);
		background: var(--background);
	}

	.icon {
		display: grid;
		flex: none;
		place-items: center;
		width: 20px;
		height: 20px;
		padding: 0;
		border: none;
		border-radius: 4px;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}

	.icon:hover:not(:disabled),
	.icon.open {
		background: var(--island-border);
		color: var(--text);
	}

	.icon.hidden {
		visibility: hidden;
	}

	svg {
		width: 13px;
		height: 13px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.history {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin: 2px 10px 6px 36px;
		padding: 0;
		list-style: none;
		font-size: 11px;
	}

	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent);
		font: inherit;
		cursor: pointer;
	}

	.link:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.history li {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.chip {
		flex: none;
		width: 12px;
		height: 12px;
		border: 1px solid var(--island-border);
		border-radius: 4px;
	}

	.check {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 6px;
		color: var(--text);
		font-size: 12px;
		white-space: nowrap;
		cursor: pointer;
	}

	.check input {
		width: auto;
	}

	.detail {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 280px;
		gap: 16px;
	}

	.stage {
		min-height: 260px;
		padding: 16px;
	}

	.stage img {
		width: min(100%, 320px);
		height: auto;
	}

	.side {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.brief {
		margin: 0;
		white-space: pre-wrap;
		font-size: 13px;
	}

	.upload {
		position: relative;
		text-align: center;
	}

	.upload input {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}

	.upload.disabled {
		opacity: 0.5;
		pointer-events: none;
	}

	.warning {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px;
		border: 1px solid var(--flag);
		border-radius: 8px;
		font-size: 12px;
	}

	.warning p {
		margin: 0;
	}

	.row {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	.versions {
		display: flex;
		flex-direction: column;
		margin: 0;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		list-style: none;
	}

	.versions li {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px 12px;
	}

	.versions li + li {
		border-top: 1px solid var(--island-border);
	}

	.versions li.current {
		background: var(--hover);
	}

	.versions img {
		width: 48px;
		height: 48px;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.versions strong {
		font-size: 13px;
		font-weight: 500;
	}

	.versions .done {
		margin-left: 6px;
	}

	.grow {
		flex: 1;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}

	input,
	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: 6px 8px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--background);
		color: var(--text);
		font: inherit;
		font-size: 13px;
	}

	.search {
		width: 180px;
	}

	textarea {
		resize: vertical;
	}

	footer {
		display: flex;
		gap: 8px;
	}

	.wide {
		flex-basis: 100%;
	}

	.error {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
	}

	.button {
		padding: 5px 10px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 12px;
		cursor: pointer;
	}

	.button:hover:not(:disabled) {
		background: var(--hover);
	}

	.button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.button.primary {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--background);
	}

	.button.danger {
		color: var(--negative);
	}

	.button.primary.danger {
		border-color: var(--negative);
		background: var(--negative);
		color: var(--background);
	}

	@media (max-width: 900px) {
		.styling {
			grid-template-columns: minmax(0, 1fr);
		}

		.palette {
			position: static;
			max-height: none;
		}
	}

	@media (max-width: 720px) {
		.detail {
			grid-template-columns: 1fr;
		}
	}
</style>
