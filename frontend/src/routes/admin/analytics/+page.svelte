<script lang="ts">
	import ActivityCard from '#lib/admin/ActivityCard.svelte';
	import ActivityDetail from '#lib/admin/ActivityDetail.svelte';
	import BarCard from '#lib/admin/BarCard.svelte';
	import BarDetail from '#lib/admin/BarDetail.svelte';
	import Dialog from '#lib/admin/Dialog.svelte';
	import SeriesCard from '#lib/admin/SeriesCard.svelte';
	import LocationCard from '#lib/admin/LocationCard.svelte';
	import LocationDetail from '#lib/admin/LocationDetail.svelte';
	import SeriesDetail from '#lib/admin/SeriesDetail.svelte';
	import {
		ACTIVITY,
		BREAKDOWNS,
		HOURS_NOTE,
		METRICS,
		format,
		funnel,
		funnelNote,
		countries,
		held,
		presetRange,
		source,
		totals,
		type Range
	} from '#lib/admin/analytics.js';
	import { params, update } from '#lib/url.svelte.js';
	import { invalidate } from '$app/navigation';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const analytics = $derived(data.analytics);
	const all = $derived(presetRange('all', analytics));

	type Breakdown = keyof typeof BREAKDOWNS;
	type Activity = keyof typeof ACTIVITY;
	type Open =
		| { kind: 'metric'; key: string }
		| { kind: 'breakdown'; key: Breakdown }
		| { kind: 'activity'; key: Activity }
		| { kind: 'guilds' }
		| { kind: 'held' }
		| { kind: 'funnel' }
		| { kind: 'location' };

	const open: Open | null = $derived.by(() => {
		const card = params().get('card');
		if (!card) return null;
		if (Object.hasOwn(METRICS, card)) return { kind: 'metric', key: card };
		if (Object.hasOwn(BREAKDOWNS, card)) return { kind: 'breakdown', key: card as Breakdown };
		const activity = card.replace(/_activity$/, '');
		if (card.endsWith('_activity') && Object.hasOwn(ACTIVITY, activity)) return { kind: 'activity', key: activity as Activity };
		if (card === 'guilds' || card === 'held' || card === 'funnel' || card === 'location') return { kind: card } as Open;
		return null;
	});

	const show = (card: string) => update({ card });
	const close = () => params().has('card') && update({ card: null, range: null, by: null, cumulative: null, compare: null });

	const sections: { title: string; metrics: string[]; breakdowns?: Breakdown[]; guilds?: boolean; held?: boolean }[] = [
		{
			title: 'Growth',
			metrics: ['signups', 'logins', 'repeat_logins', 'dau', 'referrals', 'retention_1', 'retention_7']
		},
		{ title: 'Community', metrics: ['duc', 'messages', 'feedback'], guilds: true },
		{ title: 'Building', metrics: ['projects'] },
		{
			title: 'Shipping',
			metrics: ['shipped', 'approved', 'rejected', 'pending', 'hours_approved'],
			breakdowns: ['ships_by_ring']
		},
		{ title: 'Prizes', metrics: ['outstanding', 'fulfilled'], breakdowns: ['resources', 'spent'], held: true }
	];

	const breakdownRows = (key: Breakdown, range: Range) =>
		totals(source(analytics, range).breakdowns[key], BREAKDOWNS[key].names, range);
	const guildRows = () => analytics.guilds;

	const REFRESH = 30_000;
	let refreshed = $state(Date.now());
	let now = $state(Date.now());
	let refreshing = $state(false);
	const since = $derived.by(() => {
		const seconds = Math.max(0, Math.floor((now - refreshed) / 1000));
		return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
	});

	async function refresh() {
		if (refreshing) return;
		refreshing = true;
		await invalidate('admin:analytics').finally(() => (refreshing = false));
		refreshed = now = Date.now();
	}

	$effect(() => {
		const timer = setInterval(() => {
			now = Date.now();
			if (now - refreshed >= REFRESH) refresh();
		}, 1000);
		return () => clearInterval(timer);
	});
</script>

<svelte:head>
	<title>Analytics - Hack Club Sunker Admin</title>
</svelte:head>

<header>
	<h1>Analytics</h1>
	<p>Days reset at midnight UTC. Click a card for a bigger graph and more settings.</p>
</header>

{#each sections as section (section.title)}
	<section>
		<h2>{section.title}</h2>
		<div class="grid">
			{#each section.metrics as key (key)}
				<SeriesCard
					metric={METRICS[key]}
					data={analytics}
					onopen={() => show(key)}
				/>
			{/each}
			{#if section.title === 'Building'}
				<div class="card tile">
					<span class="title">Hours spent building</span>
					<span class="value">{format(analytics.hours_building, 'hours')}</span>
					<span class="muted">Total time logged on linked projects</span>
					{#if !analytics.hours_building}<span class="muted">{HOURS_NOTE}</span>{/if}
				</div>
			{/if}
			{#each section.breakdowns ?? [] as key (key)}
				<div class="wide">
					<BarCard
						title={BREAKDOWNS[key].title}
						subtitle="All time"
						rows={breakdownRows(key, all)}
						label={BREAKDOWNS[key].label}
						onopen={() => show(key)}
					/>
				</div>
			{/each}
			{#if section.held}
				<div class="wide">
					<BarCard
						title="Resources held"
						subtitle="Now"
						rows={held(analytics)}
						label={BREAKDOWNS.resources.label}
						onopen={() => show('held')}
					/>
				</div>
			{/if}
			{#if section.guilds}
				<div class="wide">
					<BarCard
						title="Guild members"
						subtitle="Now"
						rows={guildRows()}
						onopen={() => show('guilds')}
					/>
				</div>
			{/if}
		</div>
	</section>
	{#if section.title === 'Community'}
		<section>
			<h2>Activity</h2>
			<div class="grid">
				{#each Object.keys(ACTIVITY) as Activity[] as key (key)}
					<div class="wide">
						<ActivityCard kind={key} data={analytics} range={all} onopen={() => show(`${key}_activity`)} />
					</div>
				{/each}
			</div>
		</section>
	{/if}
	{#if section.title === 'Growth'}
		<section class="narrow">
			<h2>Location</h2>
			<LocationCard rows={countries(analytics, all)} onopen={() => show('location')} paused={open !== null} />
		</section>
	{/if}
{/each}

<section>
	<h2>Review queues</h2>
	<div class="card placeholder">
		<span class="muted">Review queue stats will show up here once the review pipeline is built.</span>
	</div>
</section>

<section class="narrow">
	<h2>Funnel</h2>
	<BarCard
		title="Visitor to prize"
		subtitle="All time"
		rows={funnel(analytics, all)}
		note={funnelNote(analytics)}
		onopen={() => show('funnel')}
	/>
</section>

<footer>
	<button class="refresh" onclick={refresh} disabled={refreshing}>Refresh</button>
	<span class="muted">{since} since last refresh</span>
</footer>

{#if open?.kind === 'metric'}
	{@const metric = METRICS[open.key]}
	<Dialog title={metric.title} description={metric.description} onclose={close}>
		<SeriesDetail {metric} data={analytics} />
	</Dialog>
{:else if open?.kind === 'breakdown'}
	{@const key = open.key}
	<Dialog title={BREAKDOWNS[key].title} description={BREAKDOWNS[key].description} onclose={close}>
		<BarDetail data={analytics} rows={(range) => breakdownRows(key, range)} label={BREAKDOWNS[key].label} />
	</Dialog>
{:else if open?.kind === 'activity'}
	{@const key = open.key}
	<Dialog title={ACTIVITY[key].title} description={ACTIVITY[key].description} onclose={close}>
		<ActivityDetail kind={key} data={analytics} />
	</Dialog>
{:else if open?.kind === 'held'}
	<Dialog title="Resources held" description="Net resources across everyone's ledger right now" onclose={close}>
		<BarDetail data={analytics} rows={() => held(analytics)} label={BREAKDOWNS.resources.label} timeless />
	</Dialog>
{:else if open?.kind === 'guilds'}
	<Dialog title="Guild members" description="People in each guild right now" onclose={close}>
		<BarDetail data={analytics} rows={guildRows} timeless />
	</Dialog>
{:else if open?.kind === 'location'}
	<Dialog
		title="Where people are from"
		description="Countries from Hack Club Auth addresses, for people who signed up within the range"
		onclose={close}
	>
		<LocationDetail data={analytics} />
	</Dialog>
{:else if open?.kind === 'funnel'}
	<Dialog
		title="Funnel"
		description="People who reached each step for the first time within the range"
		onclose={close}
	>
		<BarDetail
			data={analytics}
			rows={(range) => funnel(analytics, range)}
			note={funnelNote(analytics)}
		/>
	</Dialog>
{/if}

<style>
	header {
		margin-bottom: 8px;
	}

	h1 {
		margin: 0;
		font-size: 22px;
		font-weight: 500;
	}

	header p {
		margin: 4px 0 0;
		color: var(--text-muted);
		font-size: 13px;
	}

	section {
		margin-top: 24px;
	}

	.narrow {
		max-width: 800px;
	}

	h2 {
		margin: 0 0 10px;
		color: var(--text-muted);
		font-size: 13px;
		font-weight: 500;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 8px;
	}

	.wide {
		display: grid;
		grid-column: span 2;
	}

	.card {
		padding: 16px;
		border: 1px solid var(--island-border);
		border-radius: 12px;
		background: var(--island);
	}

	.tile {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.title {
		font-size: 13px;
	}

	.value {
		font-size: 28px;
		font-weight: 500;
		line-height: 1.1;
	}

	.muted {
		color: var(--text-muted);
		font-size: 12px;
	}

	footer {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 6px;
		margin-top: 24px;
	}

	.refresh {
		padding: 6px 14px;
		border: 1px solid var(--island-border);
		border-radius: 8px;
		background: var(--island);
		color: var(--text);
		font: inherit;
		font-size: 13px;
		cursor: pointer;
		transition: border-color 0.15s;
	}

	.refresh:hover:enabled {
		border-color: var(--tip-border);
	}

	.refresh:disabled {
		color: var(--text-muted);
		cursor: default;
	}

	.refresh:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.placeholder {
		padding: 32px 16px;
		text-align: center;
	}

	@media (max-width: 560px) {
		.wide {
			grid-column: auto;
		}
	}

	@media (pointer: coarse) {
		.refresh {
			min-height: 44px;
		}
	}
</style>
