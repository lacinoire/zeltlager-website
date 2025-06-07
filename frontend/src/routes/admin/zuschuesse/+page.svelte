<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import moment from "moment";
	import type { Moment } from "moment";
	import {
		getSortByKeyFn,
		nameSortFn,
		getRegion,
		regionSortFn,
	} from "$lib/utils";
	import EditableProperty from "$lib/EditableProperty.svelte";
	import TableContainer from "$lib/TableContainer.svelte";
	import SortableTable from "$lib/SortableTable.svelte";
	import { LAGER_START } from "$lib/utils";
	import type { Column } from "$lib/utils";

	interface Member {
		id: number;
		vorname: string;
		nachname: string;
		geburtsdatum: Moment;
		strasse: string;
		hausnummer: string;
		ort: string;
		plz: string;
		alter: number;
	}

	let all: Member[];
	let filtered: Member[];
	// For sorting by region, insert empty rows
	let displayFiltered: (Member | string)[];
	let sortBy = "Vorname-asc";
	let error: string | undefined;
	let invalidAge: Member[];
	let isLoading = true;
	let regions: Map<string, Member[]>;
	let betreuer: Supervisor[];

	// &shy;
	const S = "\u00AD";

	const regionColumns: Column[] = [
		{ name: "Nr." },
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Adresse" },
		{ name: "PLZ" },
		{ name: "Ort" },
		{ name: "Alter"},
		{ name: "Unterschrift"},
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum` },
	];

	const betreuerColumns: Column[] = [
		{ name: "Nr." },
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Adresse" },
		{ name: "PLZ" },
		{ name: "Ort" },
		{ name: "Alter"},
		{ name: "Juleica-Nummer"},
		{ name: "Unterschrift"},
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum` },
	];

	function applyFilter(all: Member[], sortBy: string) {
		if (all === undefined) return;

		invalidAge = [];
		filtered = all.filter((m) => {
			if (m.alter < 6 || m.alter > 23) {
				invalidAge.push(m);
				return false;
			}
			return true;
		});

		const asc = sortBy.endsWith("asc");
		let sortFn: (a: Member, b: Member) => number;
		if (sortBy.startsWith("Name-")) {
			sortFn = (a, b) => {
				const cmp = nameSortFn(a, b);
				return asc ? cmp : -cmp;
			};
		} else if (sortBy.startsWith("Adresse-")) {
			sortFn = (a, b) => {
				const cmp = addressSortFn(a, b);
				return asc ? cmp : -cmp;
			};
		} else {
			sortFn = getSortByKeyFn(sortBy);
		}

		filtered.sort((a, b) => {
			const cmp = regionSortFn(a, b);
			if (cmp !== 0) return cmp;
			return sortFn(a, b);
		});

		regions = new Map<string, Member[]>();
		for (const e of filtered) {
			const curRegion = getRegion(parseInt(e.plz), e.ort);

			if (!regions.has(curRegion)) {
				regions.set(curRegion, [ e ]);
			} else {
				regions.get(curRegion).push(e);
			}
		}

		displayFiltered = [];
		let lastRegion = undefined;
		for (const e of filtered) {
			const curRegion = getRegion(parseInt(e.plz), e.ort);
			if (curRegion !== lastRegion) {
				const count = regions.get(curRegion).length;
				displayFiltered.push(`${curRegion} (${count} Teilnehmer)`);
			}
			displayFiltered.push(e);
			lastRegion = curRegion;
		}
	}

	$: applyFilter(all, sortBy);

	type SortTypeFn = (t: SortType) => (m: Member) => boolean;

	function createData(asDate = false) {
		const entries = [...all];

		const headers: string[] = [];
		for (const c of allColumns) {
			if ("name" in c)
				headers.push(c.name);
		}

		const data: any[] = [headers];
		for (const m of entries) {
			data.push([
				m.vorname,
				m.nachname,
				asDate ? m.geburtsdatum.toDate() : m.geburtsdatum.format("DD.MM.YYYY"),
				m.strasse + " " + m.hausnummer,
				m.ort,
				m.plz,
			]);
		}

		return data;
	}

	async function loadData() {
		const respTeilnehmer = await fetch("/api/admin/teilnehmer");
		if (!respTeilnehmer.ok) {
			// Unauthorized
			if (respTeilnehmer.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", resp);
				error = "Daten konnten nicht heruntergeladen werden. Hat der Account Admin-Rechte?";
			}
			return;
		}

		const respBetreuer = await fetch("/api/admin/betreuer");
		if (!respBetreuer.ok) {
			if (respBetreuer.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", respBetreuer);
				error = "Daten konnten nicht heruntergeladen werden. Hat der Account Admin-Rechte?";
			}
			return;
		}

		const dataTeilnehmer = await respTeilnehmer.json();
		const dataBetreuer = (await respBetreuer.json()).filter((betreuer) => {
			let startOfYear = moment().year(LAGER_START.clone().local().year() - 1).month(7).date(15);
			return startOfYear.local().diff(betreuer.anmeldedatum, 'years') < 1;
		});

		// Convert dates
		for (const e of dataTeilnehmer) {
			e.geburtsdatum = moment.utc(e.geburtsdatum).local();
			e.alter = LAGER_START.clone().local().diff(e.geburtsdatum, 'years');
		}

		for (const e of dataBetreuer) {
			e.geburtsdatum = moment.utc(e.geburtsdatum).local();
			e.alter = LAGER_START.clone().local().diff(e.geburtsdatum, 'years');
		}

		all = dataTeilnehmer;
		betreuer = dataBetreuer;
		isLoading = false;
	}

  function documentKeyDown(event) {
    if (event.key === "Escape") {
      mailModalOpen = false;
    }
  }

	onMount(loadData);
</script>

<svelte:document on:keydown={documentKeyDown} />

<svelte:head>
	<title>Zuschüsse – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

{#if error !== undefined}
	<article class="message is-danger">
		<div class="message-body">
			{error}
		</div>
	</article>
{/if}

{#if invalidAge && invalidAge.length > 0}
	<article class="message is-warn">
		<div class="content message-body">
			Folgende Teilnehmer haben ein ungültiges Alter (muss zwischen 6-23 sein):
			<ul>
			{#each invalidAge as i}
				<li>{i.vorname} {i.nachname} ({i.alter} Jahre alt)</li>
			{/each}
			</ul>
		</div>
	</article>
{/if}

{#if regions && regions.get("Außerhalb").length > 0}
	{@const teilnehmer = regions.get("Außerhalb")}
	<article class="message is-warn">
		<div class="content message-body">
			Folgende Teilnehmer sind nicht aus München und Umgebung:
			<ul>
			{#each teilnehmer as t}
				<li>{t.vorname} {t.nachname} ({t.ort})</li>
			{/each}
			</ul>
		</div>
	</article>
{/if}

{#if error === undefined && isLoading}
	<progress class="progress is-small is-primary">Loading</progress>
{/if}

{#if regions !== undefined}
	{#each [...regions] as [region, members]}
		{#if region !== "Außerhalb"}
			<h1 class="title">{region}</h1>
			<div class="nobackground">
			<TableContainer>
				<SortableTable columns={regionColumns} bind:sortBy>
					{#each members as e, i}
						<tr>
							<td>{i+1}</td>
							<td>{e.vorname}</td>
							<td>{e.nachname}</td>
							<td>{e.strasse} {e.hausnummer}</td>
							<td>{e.plz}</td>
							<td>{e.ort}</td>
							<td>{e.alter}</td>
							<td class="unterschrift"></td>
							<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
						</tr>
					{/each}
				</SortableTable>
			</TableContainer>
			</div>
			<div class="page-break"></div>
		{/if}
	{/each}
{/if}
<h1 class="title">Betreuer</h1>
<div class="nobackground">
<TableContainer>
  <SortableTable columns={betreuerColumns} bind:sortBy>
  	{#each betreuer as e, i}
			<tr>
				<td>{i+1}</td>
				<td>{e.vorname}</td>
				<td>{e.nachname}</td>
				<td>{e.strasse} {e.hausnummer}</td>
				<td>{e.plz}</td>
				<td>{e.ort}</td>
				<td>{e.alter}</td>
				<td>{e.juleica_nummer}</td>
				<td></td>
				<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
			</tr>
  	{/each}
  </SortableTable>
</TableContainer>
</div>

<style lang="scss">
	:global(.table) {
		width: 100%;
	}

	:global(.table td, .table th) {
		white-space: nowrap;
	}

	:global(.table td.unterschrift) {
		width: 99%;
	}

	@media print {

		@page {
			size: landscape
		}
		
		:global(.message) {
			display: none;
		}

		:global(.sortIcon span) {
			display: none;
		}

		:global(.page-break) {
			break-after: page;
		}

		:global(div.nobackground .table thead) {
			background: none;
		}

		:global(.table thead th button) {
			font-weight: bold;
		}

		:global(h1.title:not(:last-child)) {
			color: black;
			font-size: 1.2em;
			margin-bottom: 0.7em;
		}

		:global(.table thead) {
			background: none;
		}

		:global(.table thead th button) {
			font-weight: bold;
			font-size: 0.8em;
		}

		:global(.table td, .table thead tr th, .table tbody, .table tbody td, div.nobackground .table tbody tr:last-child td) {
			padding: 0em 0.75em;
			border-width: 2px;
			border-color: black;
			color: black;
		}

	}
</style>
