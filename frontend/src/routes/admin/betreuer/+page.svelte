<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import moment from "moment";
	import type { Moment } from "moment";
	import { addressSortFn, getSortByKeyFn, nameSortFn, createCsv, createXlsx } from "$lib/utils";
	import EditableProperty from "$lib/EditableProperty.svelte";
	import TableContainer from "$lib/TableContainer.svelte";
	import SortableTable from "$lib/SortableTable.svelte";
	import type { Column } from "$lib/utils";

	interface Supervisor {
		id: number;
		vorname: string;
		nachname: string;
		geschlecht: "Male" | "Female";
		geburtsdatum: Moment;
		juleica_nummer: string | null;
		juleica_gueltig_bis: Moment | null;
		mail: string;
		handynummer: string;
		strasse: string | null;
		hausnummer: string | null;
		ort: string | null;
		plz: string | null;
		fuehrungszeugnis_ausstellung: Moment | null;
		fuehrungszeugnis_eingesehen: Moment | null;
		krankenversicherung: string | null;
		tetanus_impfung: boolean | null;
		vegetarier: boolean | null;
		unvertraeglichkeiten: string | null;
		allergien: string | null;
		krankheiten: string | null;
		medikamente: string | null;
		kommentar: string | null;
		anmeldedatum: Moment;
	}

	let all: Supervisor[];
	let filtered: (Supervisor | string)[];
	let filter = "";
	let sortBy = "Name-asc";
	let error: string | undefined;
	let isLoading = true;

	// &shy;
	const S = "\u00AD";

	const allColumns: Column[] = [
		{ name: undefined },
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Geschlecht", displayName: "" },
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum` },
		{ name: "Juleica" },
		{ name: "Juleica gültig bis" },
		{ name: "Mail", displayName: "E-Mail" },
		{ name: "Handynummer", displayName: "Handy" },
		{ name: "Adresse" },
		{ name: "Ort" },
		{ name: "PLZ" },
		{ name: "Führungszeugnis Ausstellung", displayName: `Führungs${S}zeugnis Ausstellung` },
		{ name: "Führungszeugnis Eingesehen", displayName: `Führungs${S}zeugnis Eingesehen` },
		{ name: "Krankenversicherung", displayName: `Kranken${S}ver${S}sicherung` },
		{ name: "Tetanus-Impfung" },
		{ name: "Vegetarier" },
		{ name: "Unverträglichkeiten" },
		{ name: "Allergien" },
		{ name: "Krankheiten" },
		{ name: "Medikamente" },
		{ name: "Kommentar" },
		{ name: "Anmeldedatum" },
		{},
	];

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
		for (const [i, e] of list.entries())
			e.index = i;
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

		const headers: string[] = [];
		for (const c of allColumns) {
			if ("name" in c)
				headers.push(c.name);
		}

		const data: any[] = [headers];
		for (const m of entries) {
			const fuehrungszeugnis_ausstellung = m.fuehrungszeugnis_ausstellung
				? (asDate
					? m.fuehrungszeugnis_ausstellung.toDate()
					: m.fuehrungszeugnis_ausstellung.format("DD.MM.YYYY"))
				: m.fuehrungszeugnis_ausstellung;
			const fuehrungszeugnis_eingesehen = m.fuehrungszeugnis_eingesehen
				? (asDate
					? m.fuehrungszeugnis_eingesehen.toDate()
					: m.fuehrungszeugnis_eingesehen.format("DD.MM.YYYY"))
				: m.fuehrungszeugnis_eingesehen;
			const juleica_gueltig_bis = m.juleica_gueltig_bis
				? (asDate
					? m.juleica_gueltig_bis.toDate()
					: m.juleica_gueltig_bis.format("DD.MM.YYYY"))
				: m.juleica_gueltig_bis;

			data.push([
				m.vorname,
				m.nachname,
				m.geschlecht === "Male" ? "m" : "w",
				asDate ? m.geburtsdatum.toDate() : m.geburtsdatum.format("DD.MM.YYYY"),
				m.juleica_nummer,
				juleica_gueltig_bis,
				m.mail,
				m.handynummer,
				m.strasse + " " + m.hausnummer,
				m.ort,
				m.plz,
				fuehrungszeugnis_ausstellung,
				fuehrungszeugnis_eingesehen,
				m.krankenversicherung,
				m.tetanus_impfung,
				m.vegetarier,
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
		const resp = await fetch("/api/admin/betreuer");
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
			if (e.fuehrungszeugnis_ausstellung)
				e.fuehrungszeugnis_ausstellung = moment
					.utc(e.fuehrungszeugnis_ausstellung)
					.local();
			if (e.fuehrungszeugnis_eingesehen)
				e.fuehrungszeugnis_eingesehen = moment.utc(e.fuehrungszeugnis_eingesehen).local();
			if (e.juleica_gueltig_bis)
				e.juleica_gueltig_bis = moment.utc(e.juleica_gueltig_bis).local();
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
			if (!response.ok) error = "Betreuer konnte nicht bearbeitet werden (Server-Fehler)";
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
			if (!response.ok) error = "Betreuer konnte nicht gelöscht werden (Server-Fehler)";
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
	<SortableTable columns={allColumns} bind:sortBy>
		{#if filtered !== undefined}
			{#each filtered as e}
				<tr>
					{#if typeof e !== "string"}
						<td>{e.index + 1}</td>
						<td>{e.vorname}</td>
						<td>{e.nachname}</td>
						<td>{e.geschlecht === "Male" ? "m" : "w"}</td>
						<td>{e.geburtsdatum.format("DD.MM.YYYY")}</td>
						<td>{e.juleica_nummer ?? ""}</td>
						<td>
							{#if e.juleica_gueltig_bis !== null}
								{e.juleica_gueltig_bis.format("DD.MM.YYYY")}
							{/if}
						</td>
						<td>{e.mail}</td>
						<td>{e.handynummer}</td>
						<td>{e.strasse ?? ""} {e.hausnummer ?? ""}</td>
						<td>{e.ort ?? ""}</td>
						<td>{e.plz ?? ""}</td>
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
						<td>{e.krankenversicherung ?? ""}</td>
						<td><input type="checkbox" checked={e.tetanus_impfung} disabled /></td>
						<td><input type="checkbox" checked={e.vegetarier} disabled /></td>
						<td>{e.unvertraeglichkeiten ?? ""}</td>
						<td>{e.allergien ?? ""}</td>
						<td>{e.krankheiten ?? ""}</td>
						<td>{e.medikamente ?? ""}</td>
						<td>{e.kommentar ?? ""}</td>
						<td>{e.anmeldedatum.format("DD.MM.YY HH:mm")}</td>
						<!-- svelte-ignore a11y-invalid-attribute -->
						<td><a on:click={() => removeEntry(e)} href="#">löschen</a></td>
					{:else}
						<td colspan={allColumns.length} class="special"><h4>{e}</h4></td>
					{/if}
				</tr>
			{/each}
		{/if}
	</SortableTable>
</TableContainer>

<style>
	.header-flex {
		display: flex;
		align-items: center;
		gap: 1em;
		margin-bottom: 1em;
	}
</style>
