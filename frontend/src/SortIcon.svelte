<script lang="ts">
	import Icon from "./Icon.svelte";

	export let name: string;
	export let displayName: string | undefined = undefined;
	export let sortBy: string;

	function flipSort() {
		if (sortBy === name + "-asc") sortBy = name + "-desc";
		else sortBy = name + "-asc";
	}
</script>

<button on:click={flipSort}>
	{displayName === undefined ? name : displayName}
	{#if sortBy === name + "-asc"}
		<Icon name="sort-ascending" />
	{:else if sortBy === name + "-desc"}
		<Icon name="sort-descending" />
	{:else}
		<span class="unsorted"><Icon name="sort" /></span>
	{/if}
</button>

<style>
	button {
		border: none;
		background: none;
		color: inherit;
		font-weight: inherit;
	}

	button:hover {
		color: #000;
	}

	button:hover .unsorted {
		color: #000;
	}

	.unsorted {
		color: #888;
	}
</style>
