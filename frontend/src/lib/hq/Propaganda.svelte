<script lang="ts">
	import { request } from '#lib/joel/joel.js';

	type Video = { id: number; url: string; title: string | null; hours: number; status: 'pending' | 'approved' | 'rejected'; polled_at: string | null };
	type Propaganda = { videos: Video[]; hours: number; upgrades: number; link: string };

	let propaganda = $state<Propaganda | null>(null);
	let broken = $state(false);
	let url = $state('');
	let sending = $state(false);
	let error = $state('');
	let copied = $state(false);

	const hours = (value: number) => `${value.toFixed(1)} ${value === 1 ? 'hour' : 'hours'}`;

	function load() {
		return fetch('/api/propaganda', { cache: 'no-cache' })
			.then((response) => (response.ok ? response.json() : Promise.reject()))
			.then((data) => (propaganda = data))
			.catch(() => (broken = true));
	}

	$effect(() => {
		load();
	});

	async function submit() {
		const link = url.trim();
		if (!link || sending) return;
		sending = true;
		try {
			await request('POST', '/api/propaganda/videos', { url: link });
			url = '';
			error = '';
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		sending = false;
	}

	async function copy() {
		if (!propaganda) return;
		await navigator.clipboard.writeText(propaganda.link);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<div class="propaganda">
	<section>
		<h3>Film</h3>
		<p>Make a video for the cause of Sunker. For every 100 hours watched that your videos accumulate, you get a Ring Upgrade!</p>
		<form
			class="field"
			onsubmit={(event) => {
				event.preventDefault();
				submit();
			}}
		>
			<label class="label" for="video-url">Add video URL here:</label>
			<div class="box">
				<input id="video-url" class="input" type="url" placeholder="https://" maxlength="500" bind:value={url} />
				<button type="submit" aria-label="Submit video" disabled={sending || !url.trim()}>
					<svg viewBox="0 0 16 16"><path d="M14.5 1.5 7 9M14.5 1.5 10 14.5 7 9 1.5 6z" /></svg>
				</button>
			</div>
			{#if error}<p class="error">{error}</p>{/if}
		</form>
		{#if broken}
			<p class="muted">Couldn't load your videos</p>
		{:else if !propaganda}
			<p class="muted">Loading your videos...</p>
		{:else}
			<ul class="videos">
				{#each propaganda.videos as video (video.id)}
					<li>
						<a href={video.url} target="_blank" rel="noreferrer">{video.title || video.url}</a>
						{#if video.status === 'rejected'}
							<span class="error">Rejected</span>
						{:else if video.status === 'pending'}
							<span class="muted">Awaiting verification</span>
						{:else}
							<span class="muted">{hours(video.hours)}</span>
						{/if}
					</li>
				{:else}
					<li class="muted">No videos yet</li>
				{/each}
			</ul>
			<dl class="totals">
				<div>
					<dt>Hours watched</dt>
					<dd>{hours(propaganda.hours)}</dd>
				</div>
				<div>
					<dt>Ring Upgrades earned</dt>
					<dd>{propaganda.upgrades}</dd>
				</div>
			</dl>
		{/if}
	</section>

	<section>
		<h3>Rallying Link</h3>
		<p>Rally your friends to join the fight for Sunker! For each person that ships a project after signing up with your link, you get 10 Scrips.</p>
		<div class="box">
			<input class="input" type="text" readonly aria-label="Your rallying link" value={propaganda?.link ?? ''} onfocus={(event) => event.currentTarget.select()} />
			<button type="button" aria-label={copied ? 'Copied' : 'Copy link'} disabled={!propaganda} onclick={copy}>
				{#if copied}
					<svg viewBox="0 0 16 16"><path d="M3 8.5 6.5 12 13 4.5" /></svg>
				{:else}
					<svg viewBox="0 0 16 16"><path d="M5.5 5.5V3a1 1 0 0 1 1-1H13a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-2.5M3 5.5h6.5a1 1 0 0 1 1 1V13a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V6.5a1 1 0 0 1 1-1z" /></svg>
				{/if}
			</button>
		</div>
	</section>
</div>

<style>
	.propaganda {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	h3 {
		margin: 0;
		font: 18px 'Share Tech', sans-serif;
	}

	p {
		margin: 0;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.label {
		font-weight: 500;
	}

	.box {
		position: relative;
	}

	.box input.input {
		padding-right: 40px;
	}

	.box button {
		position: absolute;
		top: 50%;
		right: 4px;
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: none;
		border-radius: 6px;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		translate: 0 -50%;
	}

	.box button:hover:not(:disabled) {
		background: var(--hover);
		color: var(--text);
	}

	.box button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	svg {
		width: 16px;
		height: 16px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.videos {
		display: flex;
		flex-direction: column;
		margin: 0;
		padding: 0;
		border: 1px solid var(--island-border);
		border-radius: 6px;
		list-style: none;
	}

	.videos li {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		padding: 8px 12px;
	}

	.videos li + li {
		border-top: 1px solid var(--island-border);
	}

	.videos a {
		overflow: hidden;
		color: var(--text);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.videos a:hover {
		color: var(--accent);
	}

	.videos span {
		flex: none;
	}

	.totals {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
		margin: 0;
	}

	.totals div {
		padding: 10px 12px;
		border: 1px solid var(--island-border);
		border-radius: 6px;
	}

	dt {
		color: var(--text-muted);
		font: 14px 'Share Tech', sans-serif;
	}

	dd {
		margin: 4px 0 0;
		font-size: 18px;
		font-weight: 500;
	}

	.muted {
		color: var(--text-muted);
	}

	.error {
		color: var(--negative);
		font-size: 12px;
	}

	@media (pointer: coarse) {
		.box input.input {
			min-height: 44px;
			padding-right: 52px;
		}

		.box button {
			right: 2px;
			width: 40px;
			height: 40px;
		}

		.videos li {
			align-items: center;
			min-height: 44px;
			padding-block: 0;
		}

		.videos a {
			flex: 1;
			padding-block: 14px;
		}
	}
</style>
