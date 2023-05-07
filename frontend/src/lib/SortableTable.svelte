<script lang="ts">
	import SortIcon from "./SortIcon.svelte";
	import type { Column } from "./utils";

	export let columns: Column[];
	export let sortBy = "";
</script>

<table class="table is-striped is-hoverable">
	<thead class="is-sticky">
		<tr>
			{#each columns as c}
				<th>
					{#if c.name !== undefined}
						<SortIcon name={c.name} displayName={c.displayName} bind:sortBy />
					{/if}
				</th>
			{/each}
		</tr>
	</thead>
	<tbody>
		<slot />
	</tbody>
</table>
