<script lang="ts">
	import MatrixBackground from '#lib/MatrixBackground.svelte';
</script>

<svelte:head>
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono&display=swap" />
</svelte:head>

<div class="map">
	<MatrixBackground color="134 158 207" />
</div>

<style>
	.map {
		position: fixed;
		inset: 0;
		background: #0e0f10;
	}
</style>
