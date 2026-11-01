<script lang="ts">
	import { href, params } from '#lib/url.svelte.js';

	type Article = { title: string; html: string };
	type Entry = { path: string; title: string; folder: boolean };

	const ROOT = '../docs/';
	const relative = (path: string) => path.slice(ROOT.length);

	const articles: Record<string, Article> = Object.fromEntries(
		Object.entries(import.meta.glob<Article>('../docs/**/*.md', { eager: true, import: 'default' })).map(([path, article]) => [relative(path).slice(0, -'.md'.length), article])
	);
	const icons: Record<string, string> = Object.fromEntries(
		Object.entries(import.meta.glob<string>('../docs/**/icon.{svg,png,webp}', { eager: true, query: '?url', import: 'default' })).map(([path, url]) => [relative(path).replace(/\/?icon\.\w+$/, ''), url])
	);

	const name = (path: string) => path.split('/').at(-1)!.split(/[-_]/).map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
	const parent = (path: string) => path.slice(0, Math.max(0, path.lastIndexOf('/')));
	const link = (path: string) => href({ doc: path || null });

	function entries(folder: string) {
		const prefix = folder ? `${folder}/` : '';
		const found = new Map<string, Entry>();
		for (const [path, { title }] of Object.entries(articles)) {
			if (!path.startsWith(prefix)) continue;
			const [head, ...rest] = path.slice(prefix.length).split('/');
			const at = prefix + head;
			if (!found.has(at)) found.set(at, { path: at, title: rest.length ? name(head) : title, folder: rest.length > 0 });
		}
		return [...found.values()].sort((a, b) => a.title.localeCompare(b.title));
	}

	const doc = $derived(params().get('doc') ?? '');
	const article = $derived(Object.hasOwn(articles, doc) ? articles[doc] : undefined);
	const at = $derived(article || entries(doc).length ? doc : '');

	let root: HTMLElement;
	let previous: string | undefined;
	$effect(() => {
		if (previous !== undefined && previous !== doc) root.closest('aside')?.scrollTo({ top: 0 });
		previous = doc;
	});
</script>

<div class="help" bind:this={root}>
	{#if at}
		<a class="back" href={link(parent(at))} data-sveltekit-reset="false">
			<svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7" /></svg>
			{parent(at) ? name(parent(at)) : 'Help'}
		</a>
	{/if}

	{#if article}
		<article>{@html article.html}</article>
	{:else}
		{#if at}<h3>{name(at)}</h3>{/if}
		<ul>
			{#each entries(at) as entry (entry.path)}
				<li>
					<a href={link(entry.path)} data-sveltekit-reset="false">
						{#if icons[entry.path]}<img src={icons[entry.path]} alt="" />{/if}
						<span>{entry.title}</span>
						{#if entry.folder}<svg viewBox="0 0 24 24"><path d="M9 5l7 7-7 7" /></svg>{/if}
					</a>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.help {
		display: contents;
	}
	a {
		color: inherit;
		text-decoration: none;
	}

	svg {
		width: 16px;
		height: 16px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.back {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		margin: 0 0 12px -6px;
		padding: 4px 8px 4px 4px;
		border-radius: 6px;
		color: var(--text-muted);
	}

	.back:hover {
		background: var(--hover);
		color: var(--text);
	}

	h3 {
		margin: 0 0 8px;
		font: 18px 'Share Tech', sans-serif;
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li a {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border-radius: 6px;
		font-size: 14px;
	}

	li a:hover {
		background: var(--hover);
	}

	li img {
		height: 1em;
	}

	li span {
		flex: 1;
	}

	li svg {
		color: var(--text-muted);
	}

	article {
		font-size: 14px;
		line-height: 1.6;
	}

	article :global(:is(h1, h2, h3, h4, h5, h6)) {
		margin: 1.2em 0 0.4em;
		font-family: 'Share Tech', sans-serif;
		font-weight: 400;
		line-height: 1.3;
	}

	article :global(:first-child) {
		margin-top: 0;
	}

	article :global(:is(h1, h2, h3, h4, h5, h6) a) {
		color: inherit;
		text-decoration: none;
	}

	article :global(.sr-only) {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
	}

	article :global(a) {
		color: var(--accent);
	}

	article :global(:is(p, ul, ol, table, pre, blockquote)) {
		margin: 0 0 0.8em;
	}

	article :global(:is(ul, ol)) {
		padding-left: 1.4em;
	}

	article :global(.contains-task-list) {
		padding-left: 0;
		list-style: none;
	}

	article :global(code) {
		padding: 1px 4px;
		border-radius: 4px;
		background: var(--hover);
		font: 0.9em 'JetBrains Mono', monospace;
	}

	article :global(pre) {
		overflow: auto;
		padding: 10px 12px;
		border-radius: 6px;
	}

	article :global(pre code) {
		padding: 0;
		background: none;
	}

	article :global(table) {
		border-collapse: collapse;
	}

	article :global(:is(th, td)) {
		padding: 4px 10px;
		border: 1px solid var(--tip-border);
		text-align: left;
	}

	article :global(blockquote) {
		padding-left: 12px;
		border-left: 3px solid var(--tip-border);
		color: var(--text-muted);
	}

	article :global(hr) {
		border: none;
		border-top: 1px solid var(--tip-border);
	}

	article :global(img) {
		max-width: 100%;
	}

	article :global(.footnotes) {
		font-size: 12px;
		color: var(--text-muted);
	}
</style>
