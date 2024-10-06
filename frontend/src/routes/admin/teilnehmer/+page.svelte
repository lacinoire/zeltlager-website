<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import moment from "moment";
	import type { Moment } from "moment";
	import {
		addressSortFn,
		getSortByKeyFn,
		nameSortFn,
		createCsv,
		createXlsx,
		getRegion,
		regionSortFn,
	} from "$lib/utils";
	import EditableProperty from "$lib/EditableProperty.svelte";
	import TableContainer from "$lib/TableContainer.svelte";
	import SortableTable from "$lib/SortableTable.svelte";
	import type { Column } from "$lib/utils";

	interface Member {
		id: number;
		anwesend: boolean;
		bezahlt: boolean;
		vorname: string;
		nachname: string;
		geschlecht: "Male" | "Female";
		geburtsdatum: Moment;
		eltern_name: string;
		eltern_mail: string;
		eltern_handynummer: string;
		strasse: string;
		hausnummer: string;
		ort: string;
		plz: string;
		schwimmer: boolean;
		krankenversicherung: "geseztlich" | "privat" | "anderes";
		tetanus_impfung: boolean;
		ernaehrung: "alles" | "vegetarisch" | "vegan";
		unvertraeglichkeiten: string;
		allergien: string;
		krankheiten: string;
		medikamente: string;
		kommentar: string;
		anmeldedatum: Moment;
	}

	type SortType = "alphabetisch" | "region" | "anwesend" | "bezahlt";

	let all: Member[];
	let filtered: Member[];
	// For sorting by region, insert empty rows
	let displayFiltered: (Member | string)[];
	let birthdays: Member[];
	let filter = "";
	let sortBy = "Vorname-asc";
	let sortType: SortType = "alphabetisch";
	let error: string | undefined;
	let isLoading = true;

	// &shy;
	const S = "\u00AD";

	const allColumns: Column[] = [
		{ name: "Anwesend", displayName: `Anwe${S}send` },
		{ name: "Bezahlt", displayName: `Be${S}zahlt` },
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Geschlecht", displayName: "" },
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum` },
		{ name: "Eltern-Name", displayName: "Eltern" },
		{ name: "Eltern-Mail", displayName: "E-Mail" },
		{ name: "Eltern-Handynummer", displayName: "Handy" },
		{ name: "Adresse" },
		{ name: "Ort" },
		{ name: "PLZ" },
		{ name: "Schwimmer", displayName: `Schwim${S}mer` },
		{ name: "Krankenversicherung", displayName: `Kranken${S}ver${S}sicherung` },
		{ name: "Tetanus-Impfung" },
		{ name: "Ernährung" },
		{ name: "Unverträglichkeiten" },
		{ name: "Allergien" },
		{ name: "Krankheiten" },
		{ name: "Medikamente" },
		{ name: "Kommentar" },
		{ name: "Anmeldedatum" },
		{},
	];

	const regionColumns: Column[] = [
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Geschlecht", displayName: "" },
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum` },
		{ name: "Adresse" },
		{ name: "Ort" },
		{ name: "PLZ" },
	];

	function updateSortType(sortType: SortType) {
		if (sortType !== "alphabetisch" && sortType !== "region") sortBy = "Name-asc";
	}

	$: updateSortType(sortType);

	function applyFilter(all: Member[], filter: string, sortBy: string, sortType: SortType) {
		if (all === undefined) return;
		if (filter === "") {
			filtered = all;
		} else {
			filter = filter.toLowerCase();
			filtered = [];
			for (const m of all) {
				if (
					filter.length === 0 ||
					m.vorname.toLowerCase().includes(filter) ||
					m.nachname.toLowerCase().includes(filter)
				)
					filtered.push(m);
			}
		}

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

			const countPerRegion: Record<string, number> = {};
			for (const e of filtered) {
				const curRegion = getRegion(parseInt(e.plz), e.ort);
				countPerRegion[curRegion] = (countPerRegion[curRegion] ?? 0) + 1;
			}

			displayFiltered = [];
			let lastRegion = undefined;
			for (const e of filtered) {
				const curRegion = getRegion(parseInt(e.plz), e.ort);
				if (curRegion !== lastRegion) {
					const count = countPerRegion[curRegion];
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

	$: applyFilter(all, filter, sortBy, sortType);

	function isTrue(sortType: SortType) {
		return (e: Member) => (sortType === "anwesend" ? e.anwesend : e.bezahlt);
	}

	function isFalse(sortType: SortType) {
		const isT = isTrue(sortType);
		return (e: Member) => !isT(e);
	}

	const bools = [isFalse, isTrue];

	type SortTypeFn = (t: SortType) => (m: Member) => boolean;

	function boolColumns(sortType: SortType, filtered: Member[], is: SortTypeFn): Column[] {
		const num = filtered !== undefined ? filtered.filter(is(sortType)).length : "??";
		return [
			{ name: sortType === "anwesend" ? "Anwesend" : "Bezahlt" },
			{ name: "Vorname" },
			{ name: "Nachname", displayName: `Nachname (${num} Teilnehmer)` },
		];
	}

	function createData(asDate = false) {
		const entries = [...filtered];

		const headers: string[] = [];
		for (const c of allColumns) {
			if ("name" in c)
				headers.push(c.name);
		}

		const data: any[] = [headers];
		for (const m of entries) {
			data.push([
				m.anwesend,
				m.bezahlt,
				m.vorname,
				m.nachname,
				m.geschlecht === "Male" ? "m" : "w",
				asDate ? m.geburtsdatum.toDate() : m.geburtsdatum.format("DD.MM.YYYY"),
				m.eltern_name,
				m.eltern_mail,
				m.eltern_handynummer,
				m.strasse + " " + m.hausnummer,
				m.ort,
				m.plz,
				m.schwimmer,
				m.krankenversicherung,
				m.tetanus_impfung,
				m.ernaehrung,
				m.unvertraeglichkeiten,
				m.allergien,
				m.krankheiten,
				m.medikamente,
				m.kommentar,
				asDate ? m.anmeldedatum.toDate() : m.anmeldedatum.format("DD.MM.YY HH:mm"),
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
			e.anmeldedatum = moment.utc(e.anmeldedatum).local();
		}
		all = data;

		// Find birthdays
		birthdays = [];
		for (const e of data) {
			const start = moment.utc("1970-07-25").local();
			const end = moment.utc("1970-08-20").local();
			const birthday = moment(e.geburtsdatum);
			birthday.year(1970);
			// Potential birthays during the camp
			if (birthday > start && birthday < end) birthdays.push(e);
		}
		birthdays.sort((aRow, bRow) => {
			const a = moment(aRow.geburtsdatum);
			const b = moment(bRow.geburtsdatum);
			a.year(1970);
			b.year(1970);
			return a === b ? 0 : a < b ? -1 : 1;
		});
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
	<title>Teilnehmer – Zeltlager – FT München Gern e.V.</title>
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
    <li class:is-active={sortType === "bezahlt"}>
			<!-- svelte-ignore a11y-invalid-attribute -->
      <a on:click={() => sortType = "bezahlt"} href="#">
        <span>Bezahlt</span>
      </a>
    </li>
    <li class:is-active={sortType === "anwesend"}>
			<!-- svelte-ignore a11y-invalid-attribute -->
      <a on:click={() => sortType = "anwesend"} href="#">
        <span>Anwesend</span>
      </a>
    </li>
  </ul>
</div>

<div class="header-flex">
	<div class="control">
		<!-- svelte-ignore a11y-autofocus -->
		<input
			class="input"
			type="text"
			autofocus={true}
			bind:value={filter}
			placeholder="Suchen…" />
	</div>
	<div>
		{#if all !== undefined}
			{all.length} Anmeldungen
		{/if}
		<!-- svelte-ignore a11y-invalid-attribute -->
		<a on:click={() => createCsv(createData(), true)} href="#">.csv</a>
		<!-- svelte-ignore a11y-invalid-attribute -->
		<a on:click={() => createXlsx(createData(true), true)} href="#">.xlsx</a>
	</div>
</div>

{#if sortType === "alphabetisch"}
	<TableContainer>
		<SortableTable columns={allColumns} bind:sortBy>
			{#if displayFiltered !== undefined}
				{#each displayFiltered as e}
					<tr>
						{#if typeof e !== "string"}
							<td>
								<EditableProperty
									bind:value={e.anwesend}
									on:edit={(ev) => editEntry(e, ev)} />
							</td>
							<td>
								<EditableProperty
									bind:value={e.bezahlt}
									on:edit={(ev) => editEntry(e, ev)} />
							</td>
							<td>{e.vorname}</td>
							<td>{e.nachname}</td>
							<td>{e.geschlecht === "Male" ? "m" : "w"}</td>
							<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
							<td>{e.eltern_name}</td>
							<td>{e.eltern_mail}</td>
							<td>{e.eltern_handynummer}</td>
							<td>{e.strasse} {e.hausnummer}</td>
							<td>{e.ort}</td>
							<td>{e.plz}</td>
							<td><input type="checkbox" checked={e.schwimmer} disabled /></td>
							<td>{e.krankenversicherung}</td>
							<td><input type="checkbox" checked={e.tetanus_impfung} disabled /></td>
							<td>{e.ernaehrung}</td>
							<td>{e.unvertraeglichkeiten}</td>
							<td>{e.allergien}</td>
							<td>{e.krankheiten}</td>
							<td>{e.medikamente}</td>
							<td>{e.kommentar}</td>
							<td>{e.anmeldedatum.format("DD.MM.YY HH:mm")}</td>
							<!-- svelte-ignore a11y-invalid-attribute -->
							<td><a on:click={() => removeEntry(e)} href="#">löschen</a></td>
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
							<td>{e.geschlecht === "Male" ? "m" : "w"}</td>
							<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
							<td>{e.strasse} {e.hausnummer}</td>
							<td>{e.ort}</td>
							<td>{e.plz}</td>
						{:else}
							<td colspan="21" class="special"><h4>{e}</h4></td>
						{/if}
					</tr>
				{/each}
			{/if}
		</SortableTable>
	</TableContainer>
{:else}
	<div class="multiTables">
		{#each bools as is}
			<SortableTable columns={boolColumns(sortType, filtered, is)} bind:sortBy>
				{#if filtered !== undefined}
					{#each filtered as e}
						<tr class:is-hidden={!is(sortType)(e)}>
							<td>
								{#if sortType === "anwesend"}
									<EditableProperty
										bind:value={e.anwesend}
										on:edit={(ev) => editEntry(e, ev)} />
								{:else}
									<EditableProperty
										bind:value={e.bezahlt}
										on:edit={(ev) => editEntry(e, ev)} />
								{/if}
							</td>
							<td>{e.vorname}</td>
							<td>{e.nachname}</td>
						</tr>
					{/each}
				{/if}
			</SortableTable>
		{/each}
	</div>
{/if}

<h3 class="title is-3">Geburtstagskinder</h3>

<table class="table is-striped is-hoverable">
	<thead>
		<tr>
			<th>Vorname</th>
			<th>Nachname</th>
			<th><!-- Geschlecht --></th>
			<th>Geburtsdatum</th>
		</tr>
	</thead>
	<tbody>
		{#if birthdays !== undefined}
			{#each birthdays as e}
				<tr>
					<td>{e.vorname}</td>
					<td>{e.nachname}</td>
					<td>{e.geschlecht === "Male" ? "m" : "w"}</td>
					<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

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
</style>
