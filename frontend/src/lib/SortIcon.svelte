<script lang="ts">
	import { mdiSort, mdiSortAscending, mdiSortDescending } from "@mdi/js";
	import Icon from "./Icon.svelte";

	interface Props {
		name: string;
		displayName?: string | undefined;
		sortBy: string;
	}

	let { name, displayName = undefined, sortBy = $bindable() }: Props = $props();

	function flipSort() {
		if (sortBy === name + "-asc") sortBy = name + "-desc";
		else sortBy = name + "-asc";
	}
</script>

<button class="sortIcon" onclick={flipSort}>
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
