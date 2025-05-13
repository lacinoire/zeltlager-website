<script lang="ts">
	import type { Snippet } from "svelte";
	import SortIcon from "./SortIcon.svelte";
	import EditableProperty from "./EditableProperty.svelte";
	import { normalizeName } from "./utils";
	import type { Column } from "./utils";

	interface Props {
		columns: Column[];
		rows: any[];
		sortBy?: string;
		children?: Snippet;
		editable: boolean;
		onedit?: (row: any, setEnabled: (enabled: boolean) => void, rowI: number, colI: number) => void;
	}

	let { columns, rows = $bindable(), sortBy = $bindable(""), children, editable = false, onedit }: Props = $props();
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
		{#if children !== undefined}
			{@render children?.()}
		{:else}
			{#each rows as row, rowI}
				<tr>
					{#if typeof row !== "string"}
						{#each columns as col, colI}
							<td>
								{#if "render" in col}
									{@render col.render(row, rowI, colI)}
								{:else if "name" in col && normalizeName(col.name) in row}
									{@const val = row[normalizeName(col.name)]}
									{#if editable && (col.editable ?? true)}
										<EditableProperty
											bind:value={row[normalizeName(col.name)]}
											isMoment={col.isMoment}
											momentFormat={col.momentFormat}
											enumValues={col.enumValues}
											onedit={(ev) => onedit?.(row, ev, rowI, colI)} />
									{:else if col.enumValues !== undefined}
										{@const enumVal = col.enumValues.find((v) => typeof v === "string" ? v === val : v.name === val)}
										{typeof enumVal === "string" ? enumVal : (enumVal.displayName ?? enumVal.name)}
									{:else if typeof val === "string" || typeof val === "number"}
										{val}
									{:else if typeof val === "boolean"}
										<input type="checkbox" checked={val} disabled />
									{:else if col.isMoment}
										{#if val}
											{val.format(col.momentFormat ?? "DD.MM.YYYY")}
										{/if}
									{/if}
								{/if}
							</td>
						{/each}
					{:else}
						<td colspan={columns.length} class="content special"><h4>{row}</h4></td>
					{/if}
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

<style lang="scss">
	.special h4 {
		margin-bottom: 0;
	}
</style>
