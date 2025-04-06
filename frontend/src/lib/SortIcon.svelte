<script lang="ts">
	import { mdiSort, mdiSortAscending, mdiSortDescending } from "@mdi/js";
	import Icon from "./Icon.svelte";

	export let name: string;
	export let displayName: string | undefined = undefined;
	export let sortBy: string;

	function flipSort() {
		if (sortBy === name + "-asc") sortBy = name + "-desc";
		else sortBy = name + "-asc";
	}
</script>

<button class="sortIcon" on:click={flipSort}>
	{displayName === undefined ? name : displayName}
	<span>
		{#if sortBy === name + "-asc"}
			<Icon name={mdiSortAscending} />
		{:else if sortBy === name + "-desc"}
			<Icon name={mdiSortDescending} />
		{:else}
			<span class="unsorted"><Icon name={mdiSort} /></span>
		{/if}
	</span>
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
