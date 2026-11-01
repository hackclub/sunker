<script lang="ts">
	let { status }: { status: 'draft' | 'gathering' | 'completed' } = $props();
</script>

<span class="status {status}">{status}</span>

<style>
	.status {
		padding: 2px 8px;
		border: 1px solid currentColor;
		border-radius: 4px;
		color: var(--text-muted);
		font-size: 12px;
		text-transform: capitalize;
	}

	.status.gathering {
		color: var(--accent);
	}

	.status.completed {
		color: var(--positive);
	}
</style>
