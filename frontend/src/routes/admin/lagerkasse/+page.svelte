<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import moment from "moment";
	import type { Moment } from "moment";
	import {
		getSortByKeyFn,
	} from "$lib/utils";
	import EditableProperty from "$lib/EditableProperty.svelte";
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
	// For sorting by region, insert empty rows
	let sortBy = "Vorname-asc";
	// let sortType: SortType = "alphabetisch";
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
		// all = data;
		all = [
		{
			vorname: "bob",
			nachname: "almond"
		},{
			vorname: "alice",
			nachname: "Chocolate"
		},{
			vorname: "charlie",
			nachname: "brownie"
		},
		];

		all.sort(getSortByKeyFn(sortBy))

		isLoading = false;
	}

	async function editEntry(
		entry: Member,
		event: CustomEvent<{ setEnabled: (enabled: boolean) => void }>
	) {
		event.detail.setEnabled(false);
		const data = {
			member: entry.id,
			bezahlt: entry.bezahlt,
			anwesend: entry.anwesend,
		};
		try {
			const response = await fetch("/api/admin/teilnehmer/edit", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok) error = "Teilnehmer konnte nicht bearbeitet werden (Server-Fehler)";
		} catch (e) {
			console.error("Failed to edit member", e);
			error = "Teilnehmer konnte nicht bearbeitet werden";
		}
		await loadData();
		event.detail.setEnabled(true);
	}

	async function removeEntry(entry: Member) {
		if (!window.confirm(`${entry.vorname} ${entry.nachname} löschen?`)) return;
		try {
			const data = {
				member: entry.id,
			};

			const response = await fetch("/api/admin/teilnehmer/remove", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok) error = "Teilnehmer konnte nicht gelöscht werden (Server-Fehler)";
		} catch (e) {
			console.error("Failed to delete member", e);
			error = "Teilnehmer konnte nicht gelöscht werden";
		}

		await loadData();
	}

	onMount(loadData);
</script>

<svelte:head>
	<title>Lagerkasse – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

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
		{#if all !== undefined}
			{#each all as e}
				<tr>
					<td>{e.vorname}</td>
					<td>{e.nachname}</td>
					<td></td>
				</tr>
			{/each}
		{/if}
	</SortableTable>
</TableContainer>

<style lang="scss">
	.header-flex {
		display: flex;
		align-items: center;
		gap: 1em;
		margin-bottom: 1em;

		.buttons,
		.button {
			margin-bottom: 0;
		}
	}

	.multiTables {
		display: flex;
		flex-flow: row wrap;
		align-items: start;
		gap: 3em;
	}

	@media print {
		:global(.sortIcon span) {
			display: none;
		}
	}
</style>
