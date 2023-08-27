<script lang="ts">
	import { onMount } from "svelte";
	import moment from "moment";
	import type { Moment } from "moment";
	import { addressSortFn, getSortByKeyFn, nameSortFn, createCsv, createXlsx } from "$lib/utils";
	import EditableProperty from "$lib/EditableProperty.svelte";
	import SortIcon from "$lib/SortIcon.svelte";
	import TableContainer from "$lib/TableContainer.svelte";

	const headers = [
		"Vorname",
		"Nachname",
		"Geschlecht",
		"Geburtsdatum",
		"Juleica",
		"E-Mail",
		"Handynummer",
		"Straße",
		"Hausnummer",
		"Ort",
		"PLZ",
		"Führungszeugnis Ausstellung",
		"Führungszeugnis Eingesehen",
		"Krankenversicherung",
		"Tetanus-Impfung",
		"Vegetarier",
		"Allergien",
		"Unverträglichkeiten",
		"Medikamente",
		"Besonderheiten",
		"Anmeldedatum",
	];

	interface Supervisor {
		id: number;
		vorname: string;
		nachname: string;
		geschlecht: "Male" | "Female";
		geburtsdatum: Moment;
		juleica_nummer: string | null;
		mail: string;
		handynummer: string;
		strasse: string;
		hausnummer: string;
		ort: string;
		plz: string;
		fuehrungszeugnis_ausstellung: Moment | null;
		fuehrungszeugnis_eingesehen: Moment | null;
		krankenversicherung: string;
		tetanus_impfung: boolean;
		vegetarier: boolean;
		allergien: string;
		unvertraeglichkeiten: string;
		medikamente: string;
		besonderheiten: string;
		anmeldedatum: Moment;
	}

	let all: Supervisor[];
	let filtered: (Supervisor | string)[];
	let filter = "";
	let sortBy = "Name-asc";
	let error: string | undefined;
	let isLoading = true;

	function filterList(list: Supervisor[], filter: string, sortBy: string): Supervisor[] {
		if (filter === "") {
		} else {
			filter = filter.toLowerCase();
			const orig = list;
			list = [];
			for (const m of orig) {
				if (
					filter.length === 0 ||
					m.vorname.toLowerCase().includes(filter) ||
					m.nachname.toLowerCase().includes(filter)
				) {
					list.push(m);
				}
			}
		}

		const asc = sortBy.endsWith("asc");
		if (sortBy.startsWith("Name-")) {
			list.sort((a, b) => {
				const cmp = nameSortFn(a, b);
				return asc ? cmp : -cmp;
			});
		} else if (sortBy.startsWith("Adresse-")) {
			list.sort((a, b) => {
				const cmp = addressSortFn(a, b);
				return asc ? cmp : -cmp;
			});
		} else {
			list.sort(getSortByKeyFn(sortBy));
		}
		return list;
	}

	function applyFilter(all: Supervisor[], filter: string, sortBy: string) {
		if (all === undefined) return;

		filtered = [];

		function groupBy(list, callback) {
			return list.reduce((res, x) => {
				const k = callback(x);
				(res[k] = res[k] || []).push(x);
				return res;
			}, {});
		}

		// Sort and group by descending year
		const grouped = groupBy(all, (e) => {
			// Everything after xxxx-08-15 counts as next year
			const month = e.anmeldedatum.month() + 1 + (e.anmeldedatum.day() >= 15 ? 0.5 : 0);
			return e.anmeldedatum.year() + (month >= 8.5 ? 1 : 0);
		});

		const years = Object.keys(grouped);
		years.sort((a, b) => (a == b ? 0 : a > b ? -1 : 1));
		for (const year of years) {
			const yearFiltered = filterList(grouped[year], filter, sortBy);
			if (yearFiltered.length > 0) {
				filtered.push("Zeltlager " + year + " (" + grouped[year].length + " Betreuer)");
				filtered = filtered.concat(yearFiltered);
			}
		}
	}

	$: applyFilter(all, filter, sortBy);

	function createData(asDate = false) {
		const entries = [...all];
		entries.sort(nameSortFn);

		const data: any[] = [headers];
		for (const m of entries) {
			const fuehrungszeugnis_ausstellung = m.fuehrungszeugnis_ausstellung
				? asDate
					? m.fuehrungszeugnis_ausstellung.toDate()
					: m.fuehrungszeugnis_ausstellung.format("DD.MM.YYYY")
				: m.fuehrungszeugnis_ausstellung;
			const fuehrungszeugnis_eingesehen = m.fuehrungszeugnis_eingesehen
				? asDate
					? m.fuehrungszeugnis_eingesehen.toDate()
					: m.fuehrungszeugnis_eingesehen.format("DD.MM.YYYY")
				: m.fuehrungszeugnis_eingesehen;
			data.push([
				m.vorname,
				m.nachname,
				m.geschlecht === "Male" ? "m" : "w",
				asDate ? m.geburtsdatum.toDate() : m.geburtsdatum.format("DD.MM.YYYY"),
				m.juleica_nummer,
				m.mail,
				m.handynummer,
				m.strasse,
				m.hausnummer,
				m.ort,
				m.plz,
				fuehrungszeugnis_ausstellung,
				fuehrungszeugnis_eingesehen,
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
		const data = await (await fetch("/api/admin/betreuer")).json();
		// Convert dates
		for (const e of data) {
			e.geburtsdatum = moment.utc(e.geburtsdatum).local();
			e.anmeldedatum = moment.utc(e.anmeldedatum).local();
			// Also fix typo
			if (e.fuehrungszeugnis_auststellung)
				e.fuehrungszeugnis_ausstellung = moment
					.utc(e.fuehrungszeugnis_auststellung)
					.local();
			else e.fuehrungszeugnis_ausstellung = e.fuehrungszeugnis_auststellung;
			delete e.fuehrungszeugnis_auststellung;
			if (e.fuehrungszeugnis_eingesehen)
				e.fuehrungszeugnis_eingesehen = moment.utc(e.fuehrungszeugnis_eingesehen).local();
		}
		all = data;
		isLoading = false;
	}

	async function editEntry(
		entry: Supervisor,
		event: CustomEvent<{ setEnabled: (enabled: boolean) => void }>
	) {
		event.detail.setEnabled(false);
		const data = {
			supervisor: entry.id,
			juleica_nummer: entry.juleica_nummer,
			fuehrungszeugnis_ausstellung: entry.fuehrungszeugnis_ausstellung
				? entry.fuehrungszeugnis_ausstellung.format("YYYY-MM-DD")
				: entry.fuehrungszeugnis_ausstellung,
			fuehrungszeugnis_eingesehen: entry.fuehrungszeugnis_eingesehen
				? entry.fuehrungszeugnis_eingesehen.format("YYYY-MM-DD")
				: entry.fuehrungszeugnis_eingesehen,
		};
		try {
			const response = await fetch("/api/admin/betreuer/edit", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok)
				error = "Betreuer konnte nicht bearbeitet werden (Server-Fehler)";
		} catch (e) {
			console.error("Failed to edit supervisor", e);
			error = "Betreuer konnte nicht bearbeitet werden";
		}
		await loadData();
		event.detail.setEnabled(true);
	}

	async function removeEntry(entry: Supervisor) {
		if (!window.confirm(`${entry.vorname} ${entry.nachname} löschen?`)) return;
		try {
			const data = {
				supervisor: entry.id,
			};

			const response = await fetch("/api/admin/betreuer/remove", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(data),
			});
			if (!response.ok)
				error = "Betreuer konnte nicht gelöscht werden (Server-Fehler)";
		} catch (e) {
			console.error("Failed to delete supervisor", e);
			error = "Betreuer konnte nicht gelöscht werden";
		}

		await loadData();
	}

	onMount(loadData);
</script>

<svelte:head>
	<title>Betreuer – Zeltlager – FT München Gern e.V.</title>
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
		<a on:click={() => createCsv(createData(), false)} href="#">.csv</a>
		<!-- svelte-ignore a11y-invalid-attribute -->
		<a on:click={() => createXlsx(createData(true), false)} href="#">.xlsx</a>
	</div>
</div>

<TableContainer>
	<table class="table">
		<thead class="is-sticky">
			<tr>
				<th><SortIcon name="Name" bind:sortBy /></th>
				<th>
					<!-- Geschlecht --><SortIcon name="Geschlecht" displayName="" bind:sortBy />
				</th>
				<th>
					<SortIcon name="Geburtsdatum" displayName="Geburts&shy;datum" bind:sortBy />
				</th>
				<th>
					<SortIcon name="Juleica-Nummer" displayName="Juleica" bind:sortBy />
				</th>
				<th><SortIcon name="Mail" displayName="E-Mail" bind:sortBy /></th>
				<th><SortIcon name="Handynummer" displayName="Handy" bind:sortBy /></th>
				<th><SortIcon name="Adresse" bind:sortBy /></th>
				<th><SortIcon name="Ort" bind:sortBy /></th>
				<th><SortIcon name="PLZ" bind:sortBy /></th>
				<th>
					<SortIcon
						name="Führungszeugnis Ausstellung"
						displayName="Führungs&shy;zeugnis Ausstellung"
						bind:sortBy />
				</th>
				<th>
					<SortIcon
						name="Führungszeugnis Eingesehen"
						displayName="Führungs&shy;zeugnis Eingesehen"
						bind:sortBy />
				</th>
				<th>
					<SortIcon
						name="Krankenversicherung"
						displayName="Kranken&shy;ver&shy;sicherung"
						bind:sortBy />
				</th>
				<th><SortIcon name="Tetanus-Impfung" bind:sortBy /></th>
				<th>
					<SortIcon name="Vegetarier" displayName="Vege&shy;tarier" bind:sortBy />
				</th>
				<th><SortIcon name="Allergien" bind:sortBy /></th>
				<th><SortIcon name="Unverträglichkeiten" bind:sortBy /></th>
				<th><SortIcon name="Medikamente" bind:sortBy /></th>
				<th><SortIcon name="Besonderheiten" bind:sortBy /></th>
				<th><SortIcon name="Anmeldedatum" bind:sortBy /></th>
				<th />
			</tr>
		</thead>
		<tbody>
			{#if filtered !== undefined}
				{#each filtered as e}
					<tr>
						{#if typeof e !== "string"}
							<td>{e.vorname} {e.nachname}</td>
							<td>{e.geschlecht === "Male" ? "m" : "w"}</td>
							<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
							<td>
								<EditableProperty
									bind:value={e.juleica_nummer}
									on:edit={(ev) => editEntry(e, ev)} />
							</td>
							<td>{e.mail}</td>
							<td>{e.handynummer}</td>
							<td>{e.strasse} {e.hausnummer}</td>
							<td>{e.ort}</td>
							<td>{e.plz}</td>
							<td>
								<EditableProperty
									bind:value={e.fuehrungszeugnis_ausstellung}
									isMoment={true}
									on:edit={(ev) => editEntry(e, ev)} />
							</td>
							<td>
								<EditableProperty
									bind:value={e.fuehrungszeugnis_eingesehen}
									isMoment={true}
									on:edit={(ev) => editEntry(e, ev)} />
							</td>
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
							<td colspan="20" class="special"><h4>{e}</h4></td>
						{/if}
					</tr>
				{/each}
			{/if}
		</tbody>
	</table>
</TableContainer>

<style>
	.header-flex {
		display: flex;
		align-items: center;
		gap: 1em;
		margin-bottom: 1em;
	}
</style>
