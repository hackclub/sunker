<script lang="ts">
	import { onMount } from 'svelte';
	import Segmented from '#lib/admin/Segmented.svelte';
	import { request } from '#lib/joel/joel.js';

	type Status = 'pending' | 'approved' | 'rejected';
	type Person = { id: number; name: string | null; slack_id: string | null };
	type Tracked = { processing?: Record<string, unknown>; post?: Record<string, unknown>; metrics?: Record<string, unknown>; screenplay?: unknown; hack_club_segments?: unknown[] | null };
	type Video = {
		id: number;
		url: string;
		title: string | null;
		hours: number;
		status: Status;
		tracked: Tracked | null;
		polled_at: string | null;
		created_at: string;
		reviewed_at: string | null;
		user: Person;
		reviewer: Person | null;
	};

	const STATUSES: Status[] = ['pending', 'approved', 'rejected'];
	const LABELS: Record<Status, string> = { pending: 'Pending', approved: 'Approved', rejected: 'Rejected' };

	let status = $state<Status>('pending');
	let videos = $state<Video[] | null>(null);
	let counts = $state<Partial<Record<Status, number>>>({});
	let selected = $state<number | null>(null);
	let failure = $state('');
	let error = $state('');
	let busy = $state(false);

	const video = $derived(videos?.find((v) => v.id === selected) ?? null);
	const options = $derived(STATUSES.map((value) => ({ value, label: `${LABELS[value]} (${counts[value] ?? 0})` })));

	const display = (person: Person) => person.name ?? `User #${person.id}`;
	const date = (iso: string) => new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
	const humanize = (key: string) => key.replace(/_/g, ' ').replace(/^./, (c) => c.toUpperCase());
	const scalar = (value: unknown) => value === null || ['string', 'number', 'boolean'].includes(typeof value);

	function format(value: unknown): string {
		if (value === null || value === undefined) return '-';
		if (typeof value === 'number') return value.toLocaleString(undefined, { maximumFractionDigits: 2 });
		if (typeof value === 'object' && 'value' in value) return format(value.value);
		if (typeof value === 'object') return JSON.stringify(value);
		return String(value);
	}

	function observed(value: unknown) {
		return value && typeof value === 'object' && 'observed_at' in value && typeof value.observed_at === 'string' ? `Observed ${date(value.observed_at)}` : undefined;
	}

	function fields(record: Record<string, unknown> | undefined) {
		return Object.entries(record ?? {}).filter(([key, value]) => scalar(value) && value !== null && value !== '' && key !== 'title');
	}

	function embed(link: string): { src: string; tall: boolean } | null {
		let url: URL;
		try {
			url = new URL(link);
		} catch {
			return null;
		}
		const host = url.hostname.replace(/^(www|m)\./, '');
		const path = url.pathname.split('/').filter(Boolean);
		if (host === 'youtu.be' && path[0]) return { src: `https://www.youtube.com/embed/${path[0]}`, tall: false };
		if (host.endsWith('youtube.com')) {
			if (path[0] === 'shorts' && path[1]) return { src: `https://www.youtube.com/embed/${path[1]}`, tall: true };
			const id = url.searchParams.get('v') ?? (['live', 'embed'].includes(path[0]) ? path[1] : null);
			return id ? { src: `https://www.youtube.com/embed/${id}`, tall: false } : null;
		}
		if (host.endsWith('tiktok.com')) {
			const i = path.indexOf('video');
			return i >= 0 && path[i + 1] ? { src: `https://www.tiktok.com/embed/v2/${path[i + 1]}`, tall: true } : null;
		}
		if (host.endsWith('instagram.com')) {
			const i = path.findIndex((part) => ['p', 'reel', 'reels', 'tv'].includes(part));
			return i >= 0 && path[i + 1] ? { src: `https://www.instagram.com/${path[i] === 'reels' ? 'reel' : path[i]}/${path[i + 1]}/embed`, tall: true } : null;
		}
		if (host.endsWith('facebook.com') || host === 'fb.watch') {
			return { src: `https://www.facebook.com/plugins/video.php?href=${encodeURIComponent(link)}&show_text=false`, tall: false };
		}
		return null;
	}

	async function load() {
		try {
			const data: { videos: Video[]; counts: Partial<Record<Status, number>> } = await request('GET', `/api/admin/videos?status=${status}`);
			videos = data.videos;
			counts = data.counts;
			failure = '';
			if (!videos.some((v) => v.id === selected)) selected = videos[0]?.id ?? null;
		} catch (e) {
			failure = `Couldn't load videos: ${e instanceof Error ? e.message : e}`;
		}
	}

	onMount(load);

	function show(value: Status) {
		status = value;
		videos = null;
		selected = null;
		load();
	}

	async function act(action: () => Promise<unknown>, advance: boolean) {
		if (!video || busy) return;
		busy = true;
		error = '';
		const index = videos!.indexOf(video);
		try {
			await action();
			if (advance) selected = videos![index + 1]?.id ?? videos![index - 1]?.id ?? null;
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		busy = false;
	}

	const decide = (decision: Status) => act(() => request('POST', `/api/admin/videos/${video!.id}`, { status: decision }), true);
	const repoll = () => act(() => request('POST', `/api/admin/videos/${video!.id}/refresh`), false);

	function step(by: number) {
		if (!videos?.length) return;
		const index = videos.findIndex((v) => v.id === selected);
		selected = videos[Math.max(0, Math.min(videos.length - 1, index + by))].id;
	}

	function keydown(event: KeyboardEvent) {
		if (event.target instanceof HTMLElement && event.target.closest('input, textarea, select') || event.metaKey || event.ctrlKey || event.altKey) return;
		if (event.key === 'ArrowDown' || event.key === 'j') step(1);
		else if (event.key === 'ArrowUp' || event.key === 'k') step(-1);
		else if (event.key === 'a' && video && video.status !== 'approved') decide('approved');
		else if (event.key === 'r' && video?.status === 'pending') decide('rejected');
		else return;
		event.preventDefault();
	}
</script>

<svelte:head>
	<title>Video Verification - Hack Club Sunker Admin</title>
</svelte:head>

<svelte:window onkeydown={keydown} />

<div class="verification">
	<header>
		<h1>Video Verification</h1>
		<span class="muted">J / K to move, A to approve, R to reject</span>
		<Segmented {options} value={status} label="Show" onchange={show} />
	</header>

	{#if !videos}
		<p class="muted">{failure || 'Loading videos...'}</p>
	{:else if !videos.length}
		<p class="muted">{status === 'pending' ? 'No videos waiting for verification' : `No ${LABELS[status].toLowerCase()} videos`}</p>
	{:else}
		<div class="split">
			<ul class="queue">
				{#each videos as item (item.id)}
					<li>
						<button class:current={item.id === selected} onclick={() => (selected = item.id)}>
							<strong>{item.title || item.url}</strong>
							<span class="muted">{display(item.user)}, {date(item.created_at)}</span>
							<span class="hours">{format(item.hours)} hours</span>
						</button>
					</li>
				{/each}
			</ul>

			{#if video}
				{@const player = embed(video.url)}
				{@const tracked = video.tracked}
				{@const segments = tracked?.hack_club_segments}
				<article>
					<div class="top">
						<div class="player" class:tall={player?.tall}>
							{#if player}
								<iframe src={player.src} title={video.title || video.url} allow="encrypted-media; picture-in-picture; fullscreen" allowfullscreen></iframe>
							{:else}
								<a class="fallback" href={video.url} target="_blank" rel="noreferrer">Open video</a>
							{/if}
						</div>

						<div class="summary">
							<h2>{video.title || 'Untitled video'}</h2>
							<a class="link" href={video.url} target="_blank" rel="noreferrer">{video.url}</a>
							<dl class="facts">
								<dt>Submitted by</dt>
								<dd>{display(video.user)} (#{video.user.id})</dd>
								<dt>Submitted</dt>
								<dd>{date(video.created_at)}</dd>
								<dt>Last polled</dt>
								<dd>{video.polled_at ? date(video.polled_at) : 'Never'}</dd>
								{#if video.reviewer && video.reviewed_at}
									<dt>{LABELS[video.status]} by</dt>
									<dd>{display(video.reviewer)}, {date(video.reviewed_at)}</dd>
								{/if}
								{#each fields(tracked?.post) as [key, value] (key)}
									<dt>{humanize(key)}</dt>
									<dd>{format(value)}</dd>
								{/each}
							</dl>
							<div class="weighted">
								<span class="muted">Weighted watch hours</span>
								<strong>{format(video.hours)}</strong>
							</div>
							{#if error}<p class="error">{error}</p>{/if}
							<footer>
								{#if video.status === 'pending'}
									<button class="button danger" disabled={busy} onclick={() => decide('rejected')}>Reject</button>
								{/if}
								<button class="button" disabled={busy} onclick={repoll}>Refresh stats</button>
								<span class="grow"></span>
								{#if video.status !== 'approved'}
									<button class="button primary" disabled={busy} onclick={() => decide('approved')}>Approve</button>
								{/if}
							</footer>
						</div>
					</div>

					{#if !tracked}
						<p class="muted">No data from the socials API yet. Try Refresh stats once the token is set.</p>
					{:else}
						{#if tracked.processing}
							<section>
								<h3>Processing</h3>
								<dl class="facts">
									{#each fields(tracked.processing) as [key, value] (key)}
										<dt>{humanize(key)}</dt>
										<dd>{format(value)}</dd>
									{/each}
								</dl>
							</section>
						{/if}

						<section>
							<h3>Stats</h3>
							<div class="stats">
								{#each Object.entries(tracked.metrics ?? {}) as [key, value] (key)}
									<div class="stat" title={observed(value)}>
										<span class="muted">{humanize(key)}</span>
										<strong>{format(value)}</strong>
									</div>
								{:else}
									<p class="muted">No stats yet</p>
								{/each}
							</div>
						</section>

						<section>
							<h3>Hack Club segments</h3>
							{#if segments === null || segments === undefined}
								<p class="muted">Not coded yet</p>
							{:else if !segments.length}
								<p class="flag">Coded, and Hack Club never comes up in this video</p>
							{:else}
								<ul class="segments">
									{#each segments as segment, i (i)}
										<li>
											{#if segment && typeof segment === 'object'}
												<dl class="facts">
													{#each fields(segment as Record<string, unknown>) as [key, value] (key)}
														<dt>{humanize(key)}</dt>
														<dd>{format(value)}</dd>
													{/each}
												</dl>
											{:else}
												{format(segment)}
											{/if}
										</li>
									{/each}
								</ul>
							{/if}
						</section>

						{#if tracked.screenplay}
							<details>
								<summary>Screenplay</summary>
								<pre>{typeof tracked.screenplay === 'string' ? tracked.screenplay : JSON.stringify(tracked.screenplay, null, 2)}</pre>
							</details>
						{/if}
					{/if}
				</article>
			{/if}
		</div>
	{/if}
</div>

<style>
	.verification {
		display: flex;
		flex-direction: column;
		gap: 24px;
		height: 100%;
	}

	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}

	h1 {
		margin: 0;
		font-size: 20px;
		font-weight: 500;
	}

	header .muted {
		margin-right: auto;
	}

	.split {
		display: grid;
		grid-template-columns: 280px 1fr;
		gap: 16px;
		min-height: 0;
		flex: 1;
	}

	.queue {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin: 0;
		padding: 0;
		overflow: auto;
		list-style: none;
	}

	.queue button {
		display: flex;
		flex-direction: column;
		gap: 4px;
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-radius: 10px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
		text-align: left;
		cursor: pointer;
	}

	.queue button:hover {
		background: var(--hover);
	}

	.queue button.current {
		border-color: var(--accent);
	}

	.queue strong {
		overflow: hidden;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.hours {
		color: var(--accent);
		font-size: 12px;
	}

	article {
		display: flex;
		flex-direction: column;
		gap: 20px;
		min-width: 0;
		overflow: auto;
		container-type: inline-size;
	}

	.top {
		display: grid;
		grid-template-columns: minmax(0, 1.4fr) minmax(260px, 1fr);
		gap: 16px;
		align-items: start;
	}

	.player {
		overflow: hidden;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: #000;
		aspect-ratio: 16 / 9;
	}

	.player.tall {
		justify-self: center;
		width: min(100%, 340px, 60dvh);
		aspect-ratio: 9 / 16;
	}

	iframe {
		display: block;
		width: 100%;
		height: 100%;
		border: 0;
	}

	.fallback {
		display: grid;
		place-items: center;
		height: 100%;
		color: var(--accent);
	}

	.summary {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 16px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
	}

	h2 {
		margin: 0;
		font-size: 16px;
		font-weight: 500;
	}

	.link {
		overflow: hidden;
		color: var(--accent);
		font-size: 12px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 6px 12px;
		margin: 0;
		font-size: 12px;
	}

	dt {
		color: var(--text-muted);
	}

	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}

	.weighted {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		padding-top: 12px;
		border-top: 1px solid var(--island-border);
		font-size: 12px;
	}

	.weighted strong {
		color: var(--accent);
		font: 24px 'Share Tech', sans-serif;
	}

	footer {
		display: flex;
		gap: 8px;
	}

	.grow {
		flex: 1;
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	h3 {
		margin: 0;
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.stats {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
		gap: 8px;
	}

	.stat {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-radius: 10px;
		background: var(--island);
		font-size: 12px;
	}

	.stat strong {
		font: 18px 'Share Tech', sans-serif;
	}

	.segments {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.segments li {
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-radius: 10px;
		background: var(--island);
	}

	details {
		border: 1px solid var(--island-border);
		border-radius: 10px;
		background: var(--island);
	}

	summary {
		padding: 10px 12px;
		color: var(--text-muted);
		font-size: 12px;
		cursor: pointer;
	}

	pre {
		max-height: 400px;
		margin: 0;
		padding: 0 12px 12px;
		overflow: auto;
		font-size: 12px;
		white-space: pre-wrap;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
		font-size: 12px;
	}

	.flag {
		margin: 0;
		color: var(--negative);
		font-size: 12px;
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

	@container (max-width: 640px) {
		.top {
			grid-template-columns: minmax(0, 1fr);
		}
	}

	@media (max-width: 900px), (max-height: 480px) {
		.verification {
			height: auto;
		}

		.split {
			grid-template-columns: minmax(0, 1fr);
			flex: none;
		}

		.queue {
			flex-direction: row;
			scroll-snap-type: x proximity;
		}

		.queue li {
			flex: 0 0 240px;
			min-width: 0;
			scroll-snap-align: start;
		}

		article {
			overflow: visible;
		}
	}

	@media (pointer: coarse) {
		.button {
			min-height: 44px;
		}

		summary {
			padding: 14px 12px;
		}
	}
</style>
