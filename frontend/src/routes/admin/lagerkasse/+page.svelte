<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import {
		getSortByKeyFn,
	} from "$lib/utils";
	import TableContainer from "$lib/TableContainer.svelte";
	import SortableTable from "$lib/SortableTable.svelte";
	import type { Column } from "$lib/utils";

	const headers = [
		"Vorname",
		"Nachname",
		"Betrag",
	];

	interface Member {
		vorname: string;
		nachname: string;
	}

	let all: Member[];
	let displayAll: Member[];
	let sortBy = "Vorname-asc";
	let error: string | undefined;
	let isLoading = true;

	// &shy;
	const S = "\u00AD";

	const allColumns: Column[] = [
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Betrag" },
	];

	function createData(asDate = false) {
		const entries = all;

		const data: any[] = [headers];
		for (const m of entries) {
			data.push([
				m.vorname,
				m.nachname,
			]);
		}

		return data;
	}

	function applyFilter(all: Member[], sortBy: string) {
		if (all === undefined) return;

		const asc = sortBy.endsWith("asc");
		let sortFn: (a: Member, b: Member) => number;
		if (sortBy.startsWith("Name-")) {
			sortFn = (a, b) => {
				const cmp = nameSortFn(a, b);
				return asc ? cmp : -cmp;
			};
		} else {
			sortFn = getSortByKeyFn(sortBy);
		}
		all.sort(sortFn);
		displayAll = all;
	}

	$: applyFilter(all, sortBy);


	async function loadData() {
		const resp = await fetch("/api/admin/teilnehmer");
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", resp);
				error = "Daten konnten nicht heruntergeladen werden. Hat der Account Admin-Rechte?";
			}
			return;
		}
		const data = await resp.json();
		all = data;

		all.sort(getSortByKeyFn(sortBy))

		isLoading = false;
	}

	onMount(loadData);
</script>

<svelte:head>
	<title>Lagerkasse – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title">Lagerkasse</h1>

{#if error !== undefined}
	<article class="message is-danger">
		<div class="message-body">
			{error}
		</div>
	</article>
{/if}

{#if error === undefined && isLoading}
	<progress class="progress is-small is-primary">Loading</progress>
{/if}

<TableContainer>
	<SortableTable columns={allColumns} bind:sortBy>
		{#if displayAll !== undefined}
			{#each displayAll as e}
				<tr>
					<td>{e.vorname}</td>
					<td>{e.nachname}</td>
					<td class="betrag"></td>
				</tr>
			{/each}
		{/if}
	</SortableTable>
</TableContainer>

<style lang="scss">
	:global(.table) {
		width: 100%;
		border-collapse: collapse;
	}


	:global(.table td, .table th) {
		white-space: nowrap;
	}

	:global(.table td.betrag) {
		width: 99%;
	}

	@media print {
		:global(.sortIcon span) {
			display: none;
		}

		:global(h1.title) {
			color: black;
		}

		:global(.table thead) {
			background: none;
		}

		:global(.table thead th button) {
			font-weight: bold;
		}

		:global(.table td, .table thead th, .table tbody tr:last-child td) {
			padding: 0em 0.75em;
			border-width: 2px;
			border-color: black;
			color: black;
		}

	}
</style>
