<script lang="ts">
	import { onMount, untrack } from "svelte";
	import { goto } from "$app/navigation";
	import moment from "moment";
	import type { Moment } from "moment";
	import { addressSortFn, getSortByKeyFn, nameSortFn, createCsv, createXlsx } from "$lib/utils";
	import EditableProperty from "$lib/EditableProperty.svelte";
	import TableContainer from "$lib/TableContainer.svelte";
	import SortableTable from "$lib/SortableTable.svelte";
	import { groupBy } from "$lib/utils";
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
		selbsterklaerung: bool; // false if pre-signed-up
	}

	let all: Supervisor[] = $state();
	let filter = $state("");
	let sortBy = $state("Name-asc");
	let sortByPre = $state("Name-asc");
	let error: string | undefined = $state();
	let isLoading = $state(true);
	let filtered: (Supervisor | string)[][2] = $derived.by(() => {
		if (all === undefined) return [];

		let filtered = all.filter((s) => s.selbsterklaerung);
		let filteredPre = all.filter((s) => !s.selbsterklaerung);

		return [sortList(filtered, filter, sortBy), sortList(filteredPre, filter, sortBy)];
	});

	// &shy;
	const S = "\u00AD";

	const allColumns: Column[] = [
		{ editable: false, render: cellId },
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Geschlecht", displayName: "", enumValues: [{name: "Male", displayName: "m"}, {name: "Female", displayName: "w"}] },
		{ name: "Geburtsdatum", displayName: `Geburts${S}datum`, isMoment: true },
		{ name: "Juleica Nummer", displayName: "Juleica" },
		{ name: "Juleica gültig bis", isMoment: true },
		{ name: "Mail", displayName: "E-Mail" },
		{ name: "Handynummer", displayName: "Handy" },
		{ name: "Adresse", render: cellAdresse },
		{ name: "Ort" },
		{ name: "PLZ" },
		{ name: "Führungszeugnis Ausstellung", displayName: `Führungs${S}zeugnis Ausstellung`, isMoment: true },
		{ name: "Führungszeugnis Eingesehen", displayName: `Führungs${S}zeugnis Eingesehen`, isMoment: true },
		{ name: "Krankenversicherung", displayName: `Kranken${S}ver${S}sicherung`, enumValues: ["gesetzlich", "privat", "anderes"] },
		{ name: "Tetanus-Impfung" },
		{ name: "Vegetarier" },
		{ name: "Unverträglichkeiten" },
		{ name: "Allergien" },
		{ name: "Krankheiten" },
		{ name: "Medikamente" },
		{ name: "Kommentar" },
		{ name: "Anmeldedatum", isMoment: true, momentFormat: "DD.MM.YY HH:mm" },
		{ render: cellRemove },
	];

	function sortList(all: Supervisor[], filter: string, sortBy: string): (Supervisor | string)[] {
		let filtered = [];

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
		return filtered;
	}

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
		untrack(() => {
			for (const [i, e] of list.entries())
				e.index = i;
		});
		return list;
	}

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

	async function onedit(
		entry: Supervisor,
		setEnabled: (enabled: boolean) => void,
	) {
		setEnabled(false);
		const data = {...entry};

		// Convert dates
		data.geburtsdatum = data.geburtsdatum.format("YYYY-MM-DD");
		data.anmeldedatum = data.anmeldedatum.format("YYYY-MM-DD HH:mm:ss");
		if (data.fuehrungszeugnis_ausstellung)
			data.fuehrungszeugnis_ausstellung = data.fuehrungszeugnis_ausstellung.format("YYYY-MM-DD");
		else
			data.fuehrungszeugnis_ausstellung = null;
		if (data.fuehrungszeugnis_eingesehen)
			data.fuehrungszeugnis_eingesehen = data.fuehrungszeugnis_eingesehen.format("YYYY-MM-DD");
		else
			data.fuehrungszeugnis_eingesehen = null;
		if (data.juleica_gueltig_bis)
			data.juleica_gueltig_bis = data.juleica_gueltig_bis.format("YYYY-MM-DD");
		else
			data.juleica_gueltig_bis = null;

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
		setEnabled(true);
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
		<!-- svelte-ignore a11y_autofocus -->
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
		<!-- svelte-ignore a11y_invalid_attribute -->
		<a onclick={() => createCsv(createData(), false)} href="#">.csv</a>
		<!-- svelte-ignore a11y_invalid_attribute -->
		<a onclick={() => createXlsx(createData(true), false)} href="#">.xlsx</a>
	</div>
</div>

{#snippet cellId(row)}
	{row.index + 1}
{/snippet}

{#snippet cellAdresse(row, rowI, colI)}
	<span class="address">
		<EditableProperty
			bind:value={row.strasse}
			onedit={(ev) => onedit?.(row, ev, rowI, colI)} />
		<EditableProperty
			bind:value={row.hausnummer}
			onedit={(ev) => onedit?.(row, ev, rowI, colI)} />
	</span>
{/snippet}

{#snippet cellRemove(row)}
	<!-- svelte-ignore a11y_invalid_attribute -->
	<a onclick={() => removeEntry(row)} href="#">löschen</a>
{/snippet}

<TableContainer>
	<SortableTable columns={allColumns} rows={filtered[0]} editable={true} bind:sortBy {onedit} />
</TableContainer>

<h4 class="title is-4">Neue Betreuer (noch nicht angemeldet)</h4>
<TableContainer>
	<SortableTable columns={allColumns} rows={filtered[1]} editable={true} bind:sortByPre {onedit} />
</TableContainer>

<style>
	.header-flex {
		display: flex;
		align-items: center;
		gap: 1em;
		margin-bottom: 1em;
	}

	.address {
		display: flex;
		gap: 0.5em;
	}
</style>
