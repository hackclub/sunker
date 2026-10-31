<script lang="ts">
	import AuditLog from './AuditLog.svelte';
	import Permissions from './Permissions.svelte';

	let version = $state(0);
</script>

<svelte:head>
	<title>Administration - Hack Club Sunker Admin</title>
</svelte:head>

<div class="administration">
	<h1>Administration</h1>
	<Permissions onchange={() => version++} />
	<AuditLog {version} />
</div>

<style>
	.administration {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	h1 {
		margin: 0;
		font-size: 20px;
		font-weight: 500;
	}
</style>
