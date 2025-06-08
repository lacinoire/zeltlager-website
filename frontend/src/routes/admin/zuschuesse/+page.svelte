<script lang="ts">
	import { onMount, untrack } from "svelte";
	import { goto } from "$app/navigation";
	import moment from "moment";
	import type { Moment } from "moment";
	import {
		getSortByKeyFn,
		nameSortFn,
		getRegion,
		regionSortFn,
	} from "$lib/utils";
	import TableContainer from "$lib/TableContainer.svelte";
	import SortableTable from "$lib/SortableTable.svelte";
	import { LAGER_START, groupBy } from "$lib/utils";
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

	type Supervisor = Member & { juleica: string; };

	// Categories of notes that are displayed at the top
	const NoteDescriptions = [
		{
			name: "Age",
			desc: "Diese Teilnehmer haben ein ungültiges Alter (muss zwischen 6-23 sein)",
			getNote: (m) => `${m.alter} Jahre alt`,
		},
		{
			name: "PreSignup",
			desc: "Diese Betreuer sind nicht vollständig angemeldet",
		},
		{
			name: "MissingJuleica",
			desc: "Diese Betreuer haben keine Juleica Nummer",
		},
		{
			name: "OutdatedJuleica",
			desc: "Diese Betreuer haben eine abgelaufene Juleica",
			getNote: (m) => `gültig bis ${moment.utc(m.juleica_gueltig_bis).local().format("DD.MM.YYYY")}`,
		},
		{
			name: "FuehrungszeugnisEingesehen",
			desc: "Diese Betreuer haben kein eingesehenes Führungszeugnis",
		},
	];

	const Notes = Object.freeze(Object.fromEntries(NoteDescriptions.map((n, i) => [n.name, i])));

	interface Lists {
		members: Member[];
		supervisors: Supervisor[];

		notes: Member[][];
	}

	let sortBy = $state("Vorname-asc");
	let error: string | undefined = $state();
	let isLoading = $state(true);

	let mixedLists: Lists = $state(emptyLists());
	let sortedLists: Lists = $derived(sortLists(mixedLists));

	let regions = $derived.by(() => {
		const groups = groupBy(sortedLists.members, (e) => getRegion(parseInt(e.plz), e.ort));
		return ["München", "Landkreis München", "Außerhalb"].map((name) => [name, groups[name]]);
	});

	const memberColumns: Column[] = [
		{ name: "Nr.", render: cellId },
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Adresse", render: cellAdresse },
		{ name: "PLZ" },
		{ name: "Ort" },
		{ name: "Alter" },
		{ name: "Unterschrift" },
	];

	const supervisorColumns: Column[] = [
		{ name: "Nr.", render: cellId },
		{ name: "Vorname" },
		{ name: "Nachname" },
		{ name: "Adresse", render: cellAdresse },
		{ name: "PLZ" },
		{ name: "Ort" },
		{ name: "Alter" },
		{ name: "Juleica-Nummer" },
		{ name: "Unterschrift" },
	];

	function emptyLists(): Lists {
		return {
			members: [],
			supervisors: [],

			notes: Object.keys(NoteDescriptions).map(() => []),
		};
	}

	// 1. Fetch and convert dates
	async function loadData(): [Member[], Supervisor[]] {
		const respTeilnehmer = await fetch("/api/admin/teilnehmer");
		if (!respTeilnehmer.ok) {
			// Unauthorized
			if (respTeilnehmer.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", resp);
				error = "Daten konnten nicht heruntergeladen werden. Hat der Account Admin-Rechte?";
			}
			return [[], []];
		}
		const members = await respTeilnehmer.json();

		// Convert dates
		for (const e of members) {
			e.geburtsdatum = moment.utc(e.geburtsdatum).local();
			e.alter = LAGER_START.clone().local().diff(e.geburtsdatum, 'years');
		}

		const respSupervisors = await fetch("/api/admin/betreuer");
		if (!respSupervisors.ok) {
			if (respSupervisors.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", respSupervisors);
				error = "Daten konnten nicht heruntergeladen werden. Hat der Account Admin-Rechte?";
			}
			return [[], []];
		}
		const supervisors = await respSupervisors.json();

		// Convert dates
		for (const e of supervisors) {
			e.geburtsdatum = moment.utc(e.geburtsdatum).local();
			e.alter = LAGER_START.clone().local().diff(e.geburtsdatum, 'years');
		}

		isLoading = false;
		return [members, supervisors];
	}

	// 2. Mix into various lists (mixedLists)
	function mixData(members: Member[], supervisors: Supervisor[]): Lists {
		const lists = emptyLists();

		// Supervisors that should be converted first
		const firstSupervisors = [];

		let startOfYear = LAGER_START.clone().local().subtract(1, "years").add(15, "days");
		lists.supervisors = supervisors.filter((e) => {
			// Skip old signups
			if (!startOfYear.isBefore(moment.utc(e.anmeldedatum).local()))
				return false;

			// Skip pre-signups
			if (e.selbsterklaerung !== true) {
				lists.notes[Notes.PreSignup].push(e);
				return false;
			}
			if (e.fuehrungszeugnis_eingesehen === null)
				lists.notes[Notes.FuehrungszeugnisEingesehen].push(e);
			// Empty Juleicas
			if (["", "0", null].includes(e.juleica_nummer)) {
				firstSupervisors.push(e);
				lists.notes[Notes.MissingJuleica].push(e);
				return false;
			}
			// Outdated Juleicas
			if (e.juleica_gueltig_bis === null ||
			  moment.utc(e.juleica_gueltig_bis).local().isBefore(LAGER_START.clone().local())) {
				firstSupervisors.push(e);
				lists.notes[Notes.OutdatedJuleica].push(e);
				return false;
			}
			return true;
		});

		lists.members = members.filter((e) => {
			if (e.alter < 6 || e.alter > 23) {
				lists.notes[Notes.Age].push(e);
				return false;
			}
			return true;
		});

		for (const e of firstSupervisors.slice()) {
			// Not part of supervisors, so do not need to subtract one
			if (lists.members.length + 1 > 15 * lists.supervisors.length)
				break;

			if (e.alter <= 23 && getRegion(parseInt(e.plz), e.ort) !== "Außerhalb") {
				let i = firstSupervisors.indexOf(e);
				lists.members.push(e);
				firstSupervisors.splice(i, 1);
			}
		}

		// Take supervisors to members, first munich, then region
		for (const region of ["München", "Landkreis München"]) {
			for (const e of lists.supervisors.slice()) {
				if (lists.members.length + 1 > 15 * (lists.supervisors.length - 1))
					break;
				if (getRegion(parseInt(e.plz), e.ort) !== region)
					continue;

				if (e.alter <= 23) {
					let i = lists.supervisors.indexOf(e);
					lists.members.push(e);
					lists.supervisors.splice(i, 1);
				}
			}
		}

		return lists;
	}

	// 3. Sort (sortedLists)
	function sortLists(lists: Lists): Lists {
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

		untrack(() => {
			lists.members.sort(getSortByKeyFn(sortBy));
			lists.supervisors.sort(getSortByKeyFn(sortBy));
		});

		return lists;
	}

	onMount(async () => {
		const data = await loadData();
		mixedLists = mixData(data[0], data[1]);
	});
</script>

<svelte:head>
	<title>Zuschüsse – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

{#snippet cellId(row, rowI)}
	{rowI + 1}
{/snippet}

{#snippet cellAdresse(row)}
	{row.strasse} {row.hausnummer}
{/snippet}

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

{#each sortedLists.notes as note, i}
	{#if note.length > 0}
		{@const desc = NoteDescriptions[i]}
		<article class="message is-warning">
			<div class="message-header">
				{desc.desc}
			</div>
			<div class="content message-body">
				<ul>
					{#each note as member}
						<li>
							{member.vorname} {member.nachname}
							{#if "getNote" in desc}
								({desc.getNote(member)})
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		</article>
	{/if}
{/each}

{#each regions as [region, members]}
	<h1 class="title">{region}</h1>
	<div class="nobackground">
		<TableContainer>
			<SortableTable columns={memberColumns} rows={members} bind:sortBy />
		</TableContainer>
	</div>
	<div class="page-break"></div>
{/each}

<h1 class="title">Betreuer</h1>
<div class="nobackground">
	<TableContainer>
		<SortableTable columns={supervisorColumns} rows={sortedLists.supervisors} bind:sortBy />
	</TableContainer>
</div>

<style lang="scss">
	:global(.table) {
		width: 100%;
	}

	:global(.table td, .table th) {
		white-space: nowrap;
	}

	/* Unterschrift */
	:global(.table td:last-child) {
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
