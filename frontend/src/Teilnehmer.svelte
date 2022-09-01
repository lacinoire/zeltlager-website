<script lang="ts">
	import { onMount } from "svelte";
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
	} from "./utils";
	import EditableProperty from "./EditableProperty.svelte";
	import GlobalCss from "./GlobalCss.svelte";
	import SortIcon from "./SortIcon.svelte";

	const headers = [
		"Anwesend",
		"Bezahlt",
		"Vorname",
		"Nachname",
		"Geschlecht",
		"Geburtsdatum",
		"Eltern",
		"E-Mail",
		"Handynummer",
		"Straße",
		"Hausnummer",
		"Ort",
		"PLZ",
		"Schwimmer",
		"Krankenversicherung",
		"Tetanus-Impfung",
		"Vegetarier",
		"Allergien",
		"Unverträglichkeiten",
		"Medikamente",
		"Besonderheiten",
		"Anmeldedatum",
	];

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
		krankenversicherung: string;
		tetanus_impfung: boolean;
		vegetarier: boolean;
		allergien: string;
		unvertraeglichkeiten: string;
		medikamente: string;
		besonderheiten: string;
		anmeldedatum: Moment;
	}

	type SortType = "alphabetisch" | "region" | "anwesend" | "bezahlt";

	let all: Member[];
	let filtered: Member[];
	// For sorting by region, insert empty rows
	let displayFiltered: (Member | string)[];
	let birthdays: Member[];
	let filter = "";
	let sortBy = "Name-asc";
	let sortType: SortType = "alphabetisch";

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

			displayFiltered = [];
			let lastRegion = undefined;
			for (const e of filtered) {
				const curRegion = getRegion(e.plz, e.ort);
				if (curRegion !== lastRegion) displayFiltered.push(curRegion);
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

	function createData(asDate = false) {
		const entries = [...all];
		entries.sort(nameSortFn);

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
				m.strasse,
				m.hausnummer,
				m.ort,
				m.plz,
				m.schwimmer,
				m.krankenversicherung,
				m.tetanus_impfung,
				m.vegetarier,
				m.allergien,
				m.unvertraeglichkeiten,
				m.medikamente,
				m.besonderheiten,
				asDate ? m.anmeldedatum.toDate() : m.anmeldedatum.format("DD.MM.YY HH:mm"),
			]);
		}

		return data;
	}

	async function loadData() {
		const data = await (await fetch("/admin/teilnehmer.json")).json();
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
	}

	async function editEntry(entry: Member) {
		const data = {
			member: entry.id,
			bezahlt: entry.bezahlt,
			anwesend: entry.anwesend,
		};
		try {
			const response = await fetch("/admin/teilnehmer/edit", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok)
				alert("Fehler: Teilnehmer konnte nicht bearbeitet werden (Server-Fehler)");
		} catch (e) {
			console.error("Failed to edit member", e);
			alert("Fehler: Teilnehmer konnte nicht bearbeitet werden");
		}
		await loadData();
	}

	async function removeEntry(entry: Member) {
		if (!window.confirm(`${entry.vorname} ${entry.nachname} löschen?`)) return;
		try {
			const data = {
				member: entry.id,
			};

			const response = await fetch("/admin/teilnehmer/remove", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok)
				alert("Fehler: Teilnehmer konnte nicht gelöscht werden (Server-Fehler)");
		} catch (e) {
			console.error("Failed to delete member", e);
			alert("Fehler: Teilnehmer konnte nicht gelöscht werden");
		}

		await loadData();
	}

	onMount(loadData);
</script>

<GlobalCss />
<div class="form-inline">
	<div class="form-group mx-sm-3 mb-2">
		<!-- svelte-ignore a11y-autofocus -->
		<input
			class="form-control"
			type="text"
			autofocus={true}
			bind:value={filter}
			placeholder="Suchen…"
		/>
	</div>
	<div class="btn-group btn-group-toggle mb-2">
		<label class="btn btn-secondary" class:active={sortType === "alphabetisch"}>
			<input type="radio" autocomplete="off" bind:group={sortType} value="alphabetisch" /> Alphabetisch
		</label>
		<label class="btn btn-secondary" class:active={sortType === "region"}>
			<input type="radio" autocomplete="off" bind:group={sortType} value="region" /> München/Landkreis
		</label>
		<label class="btn btn-secondary" class:active={sortType === "bezahlt"}>
			<input type="radio" autocomplete="off" bind:group={sortType} value="bezahlt" /> Bezahlt
		</label>
		<label class="btn btn-secondary" class:active={sortType === "anwesend"}>
			<input type="radio" autocomplete="off" bind:group={sortType} value="anwesend" /> Anwesend
		</label>
	</div>
	<div class="mx-sm-3 mb-2">
		<!-- svelte-ignore a11y-invalid-attribute -->
		<a on:click={() => createCsv(createData(), true)} href="#">.csv</a>
		<!-- svelte-ignore a11y-invalid-attribute -->
		<a on:click={() => createXlsx(createData(true), true)} href="#">.xlsx</a>
	</div>
</div>

{#if sortType === "alphabetisch" || sortType === "region"}
	<table class="table">
		<thead class="thead-light">
			<tr>
				<th scope="col">
					<SortIcon name="Anwesend" displayName="Anwe&shy;send" bind:sortBy />
				</th>
				<th scope="col">
					<SortIcon name="Bezahlt" displayName="Be&shy;zahlt" bind:sortBy />
				</th>
				<th scope="col"><SortIcon name="Name" bind:sortBy /></th>
				<th scope="col">
					<!-- Geschlecht --><SortIcon name="Geschlecht" displayName="" bind:sortBy />
				</th>
				<th scope="col">
					<SortIcon name="Geburtsdatum" displayName="Geburts&shy;datum" bind:sortBy />
				</th>
				<th scope="col">
					<SortIcon name="Eltern-Name" displayName="Eltern" bind:sortBy />
				</th>
				<th scope="col">
					<SortIcon name="Eltern-Mail" displayName="E-Mail" bind:sortBy />
				</th>
				<th scope="col">
					<SortIcon name="Eltern-Handynummer" displayName="Handy" bind:sortBy />
				</th>
				<th scope="col"><SortIcon name="Adresse" bind:sortBy /></th>
				<th scope="col"><SortIcon name="Ort" bind:sortBy /></th>
				<th scope="col"><SortIcon name="PLZ" bind:sortBy /></th>
				<th scope="col">
					<SortIcon name="Schwimmer" displayName="Schwim&shy;mer" bind:sortBy />
				</th>
				<th scope="col">
					<SortIcon
						name="Krankenversicherung"
						displayName="Kranken&shy;ver&shy;sicherung"
						bind:sortBy
					/>
				</th>
				<th scope="col"><SortIcon name="Tetanus-Impfung" bind:sortBy /></th>
				<th scope="col">
					<SortIcon name="Vegetarier" displayName="Vege&shy;tarier" bind:sortBy />
				</th>
				<th scope="col"><SortIcon name="Allergien" bind:sortBy /></th>
				<th scope="col"><SortIcon name="Unverträglichkeiten" bind:sortBy /></th>
				<th scope="col"><SortIcon name="Medikamente" bind:sortBy /></th>
				<th scope="col"><SortIcon name="Besonderheiten" bind:sortBy /></th>
				<th scope="col"><SortIcon name="Anmeldedatum" bind:sortBy /></th>
				<th scope="col"><!-- löschen --></th>
			</tr>
		</thead>
		<tbody>
			{#if displayFiltered !== undefined}
				{#each displayFiltered as e}
					<tr>
						{#if typeof e !== "string"}
							<td>
								<EditableProperty
									bind:value={e.anwesend}
									on:edit={() => editEntry(e)}
								/>
							</td>
							<td>
								<EditableProperty
									bind:value={e.bezahlt}
									on:edit={() => editEntry(e)}
								/>
							</td>
							<td>{e.vorname} {e.nachname}</td>
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
							<td><input type="checkbox" checked={e.vegetarier} disabled /></td>
							<td>{e.allergien}</td>
							<td>{e.unvertraeglichkeiten}</td>
							<td>{e.medikamente}</td>
							<td>{e.besonderheiten}</td>
							<td>{e.anmeldedatum.format("DD.MM.YY HH:mm")}</td>
							<!-- svelte-ignore a11y-invalid-attribute -->
							<td><a on:click={() => removeEntry(e)} href="#">löschen</a></td>
						{:else}
							<td colspan="21"><h4>{e}</h4></td>
						{/if}
					</tr>
				{/each}
			{/if}
		</tbody>
	</table>
{:else}
	<div class="multiTables">
		{#each bools as is}
			<table class="table">
				<thead class="thead-light">
					<tr>
						<th scope="col">
							{#if sortType === "anwesend"}
								Anwesend
							{:else}
								Bezahlt
							{/if}
						</th>
						<th scope="col">
							Name ({filtered !== undefined
								? filtered.filter(is(sortType)).length
								: "??"}
							Teilnehmer)
						</th>
					</tr>
				</thead>
				<tbody>
					{#if filtered !== undefined}
						{#each filtered as e}
							{#if is(sortType)(e)}
								<tr>
									<td>
										{#if sortType === "anwesend"}
											<EditableProperty
												bind:value={e.anwesend}
												on:edit={() => editEntry(e)}
											/>
										{:else}
											<EditableProperty
												bind:value={e.bezahlt}
												on:edit={() => editEntry(e)}
											/>
										{/if}
									</td>
									<td>{e.vorname} {e.nachname}</td>
								</tr>
							{/if}
						{/each}
					{/if}
				</tbody>
			</table>
		{/each}
	</div>
{/if}

<h3>Geburtstagskinder</h3>

<table class="table">
	<thead class="thead-light">
		<tr>
			<th scope="col">Name</th>
			<th scope="col"><!-- Geschlecht --></th>
			<th scope="col">Geburtsdatum</th>
		</tr>
	</thead>
	<tbody>
		{#if birthdays !== undefined}
			{#each birthdays as e}
				<tr>
					<td>{e.vorname} {e.nachname}</td>
					<td>{e.geschlecht === "Male" ? "m" : "w"}</td>
					<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

<style>
	:global(.container) {
		max-width: 100%;
	}

	thead {
		position: sticky;
		top: 0;
	}

	.multiTables {
		display: flex;
		flex-flow: row wrap;
		align-items: start;
		gap: 3em;
	}

	.table {
		width: inherit;
	}
</style>
