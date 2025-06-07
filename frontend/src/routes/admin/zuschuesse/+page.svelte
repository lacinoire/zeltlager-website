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

	type SortType = "alphabetisch" | "region";

	let all: Member[];
	let filtered: Member[];
	// For sorting by region, insert empty rows
	let displayFiltered: (Member | string)[];
	let sortBy = "Vorname-asc";
	let sortType: SortType = "alphabetisch";
	let error: string | undefined;
	let invalidAge: Member[];
	let isLoading = true;
	let regions: Map<string, Member[]>;

	// &shy;
	const S = "\u00AD";

	const allColumns: Column[] = [
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Adresse" },
		{ name: "Ort" },
		{ name: "PLZ" },
		{ name: "Alter"},
		{ name: "Unterschrift"},
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum` },
	];

	const regionColumns: Column[] = [
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Adresse" },
		{ name: "Ort" },
		{ name: "PLZ" },
		{ name: "Alter"},
		{ name: "Unterschrift"},
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum` },
	];

	function updateSortType(sortType: SortType) {
		if (sortType !== "alphabetisch" && sortType !== "region") sortBy = "Name-asc";
	}

	$: updateSortType(sortType);

	function applyFilter(all: Member[], sortBy: string, sortType: SortType) {
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

		if (sortType === "region") {
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
		} else {
			filtered.sort(sortFn);
			displayFiltered = filtered;
		}
	}

	$: applyFilter(all, sortBy, sortType);

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

		// Convert dates
		for (const e of data) {
			e.geburtsdatum = moment.utc(e.geburtsdatum).local();
			e.alter = LAGER_START.clone().local().diff(e.geburtsdatum, 'years');
		}
		all = data;
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

{#if sortType === "region" && regions.get("Außerhalb").length > 0}
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

<div class="tabs">
  <ul>
    <li class:is-active={sortType === "alphabetisch"}>
			<!-- svelte-ignore a11y-invalid-attribute -->
      <a on:click={() => sortType = "alphabetisch"} href="#">
        <span>Alphabetisch</span>
      </a>
    </li>
    <li class:is-active={sortType === "region"}>
			<!-- svelte-ignore a11y-invalid-attribute -->
      <a on:click={() => sortType = "region"} href="#">
        <span>München/Landkreis</span>
      </a>
    </li>
  </ul>
</div>

{#if sortType === "alphabetisch"}
	<TableContainer>
		<SortableTable columns={allColumns} bind:sortBy>
			{#if displayFiltered !== undefined}
				{#each displayFiltered as e}
					<tr>
						{#if typeof e !== "string"}
							<td>{e.vorname}</td>
							<td>{e.nachname}</td>
							<td>{e.strasse} {e.hausnummer}</td>
							<td>{e.ort}</td>
							<td>{e.plz}</td>
							<td>{e.alter}</td>
							<td></td>
							<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
						{:else}
							<td colspan="23" class="special"><h4>{e}</h4></td>
						{/if}
					</tr>
				{/each}
			{/if}
		</SortableTable>
	</TableContainer>
{:else if sortType === "region"}
	<TableContainer>
		<SortableTable columns={regionColumns} bind:sortBy>
			{#if displayFiltered !== undefined}
				{#each displayFiltered as e}
					<tr>
						{#if typeof e !== "string"}
							<td>{e.vorname}</td>
							<td>{e.nachname}</td>
							<td>{e.strasse} {e.hausnummer}</td>
							<td>{e.ort}</td>
							<td>{e.plz}</td>
							<td>{e.alter}</td>
							<td></td>
							<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
						{:else}
							<td colspan="21" class="special"><h4>{e}</h4></td>
						{/if}
					</tr>
				{/each}
			{/if}
		</SortableTable>
	</TableContainer>
{/if}

<style lang="scss">
</style>
