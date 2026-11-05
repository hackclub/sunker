<script lang="ts">
	import { RATE } from '#lib/robosquirrel.js';

	let { text, done = $bindable(false) }: { text: string; done?: boolean } = $props();

	const parts = $derived(text.split('*').map((value, i) => ({ value, emphasis: i % 2 === 1 })));
	const length = $derived(parts.reduce((total, part) => total + part.value.length, 0));
	let shown = $state(0);

	$effect(() => {
		length;
		shown = 0;
		done = false;
		const start = performance.now();
		let frame = requestAnimationFrame(function step(now) {
			shown = Math.min(length, Math.floor((now - start) * RATE));
			if (shown < length) frame = requestAnimationFrame(step);
			else done = true;
		});
		return () => cancelAnimationFrame(frame);
	});

	export function finish() {
		shown = length;
		done = true;
	}

	const visible = $derived.by(() => {
		let left = shown;
		return parts.map((part) => {
			const value = part.value.slice(0, Math.max(0, left));
			left -= part.value.length;
			return { ...part, value };
		});
	});
</script>

<p class="speech" aria-live="polite">
	<span class="sr">{parts.map((part) => part.value).join('')}</span>
	<span class="text" aria-hidden="true">
		<span class="full">{#each parts as part, i (i)}{#if part.emphasis}<em>{part.value}</em>{:else}{part.value}{/if}{/each}</span>
		<span class="typed">{#each visible as part, i (i)}{#if part.emphasis}<em>{part.value}</em>{:else}{part.value}{/if}{/each}</span>
	</span>
	<span class="more" class:done aria-hidden="true"></span>
</p>

<style>
	.speech {
		position: relative;
		display: flex;
		align-items: flex-end;
		gap: 12px;
		margin: 0;
		font: 18px/1.45 'Inter', sans-serif;
	}

	.sr {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
	}

	.text {
		display: grid;
		flex: 1;
	}

	.full,
	.typed {
		grid-area: 1 / 1;
	}

	.full {
		visibility: hidden;
	}

	.more {
		flex-shrink: 0;
		width: 0;
		height: 0;
		margin-bottom: 0.5lh;
		border: 6px solid transparent;
		border-top-color: var(--accent);
		border-bottom: none;
		visibility: hidden;
		animation: nudge 1s ease-in-out infinite;
	}

	.more.done {
		visibility: visible;
	}

	@keyframes nudge {
		50% {
			transform: translateY(3px);
		}
	}
</style>
