<script lang="ts">
	import type { Snippet } from "svelte";
	import SortIcon from "./SortIcon.svelte";
	import type { Column } from "./utils";

	interface Props {
		columns: Column[];
		sortBy?: string;
		children?: Snippet;
	}

	let { columns, sortBy = $bindable(""), children }: Props = $props();
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
		{@render children?.()}
	</tbody>
</table>
