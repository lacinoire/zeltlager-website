<script lang="ts">
	import { onMount } from "svelte";
	import moment from "moment";
	import { nameSortFn } from "./utils";

	let allSupervisors;

  function createSupervisorData(asDate = false) {
  	let members = [];
  	for (let m of allSupervisors)
  		members.push(m);
  	members.sort(nameSortFn);

  	let data = [["Vorname", "Nachname", "Geschlecht", "Geburtsdatum", "JuLeiCa",
  		"E-Mail", "Handynummer", "Straße", "Hausnummer", "Ort", "PLZ",
  		"Führungszeugnis Ausstellung", "Führungszeugnis Eingesehen", "Krankenversicherung",
  		"Tetanus-Impfung", "Vegetarier", "Allergien", "Unverträglichkeiten", "Medikamente",
  		"Besonderheiten", "Anmeldedatum"]];
  	for (let m of members) {
  		const geburtsdatum = moment.utc(m.geburtsdatum).local();
  		const anmeldedatum = moment.utc(m.anmeldedatum).local();
  		let fuehrungszeugnis_ausstellung = moment.utc(m.fuehrungszeugnis_auststellung).local();
  		let fuehrungszeugnis_eingesehen = moment.utc(m.fuehrungszeugnis_eingesehen).local();
  		fuehrungszeugnis_ausstellung = m.fuehrungszeugnis_auststellung ? (asDate ? fuehrungszeugnis_ausstellung.toDate() : fuehrungszeugnis_ausstellung.format("DD.MM.YYYY")) : m.fuehrungszeugnis_auststellung;
  		fuehrungszeugnis_eingesehen = m.fuehrungszeugnis_eingesehen ? (asDate ? fuehrungszeugnis_eingesehen.toDate() : fuehrungszeugnis_eingesehen.format("DD.MM.YYYY")) : m.fuehrungszeugnis_eingesehen;
  		data.push([m.vorname, m.nachname, m.geschlecht === "Male" ? "m" : "w",
  			asDate ? geburtsdatum.toDate() : geburtsdatum.format("DD.MM.YYYY"),
  			m.juleica_nummer, m.mail, m.handynummer, m.strasse, m.hausnummer,
  			m.ort, m.plz, fuehrungszeugnis_ausstellung, fuehrungszeugnis_eingesehen,
  			m.krankenversicherung, boolToStr(m.tetanus_impfung), boolToStr(m.vegetarier), m.allergien,
  			m.unvertraeglichkeiten, m.medikamente, m.besonderheiten,
  			asDate ? anmeldedatum.toDate() : anmeldedatum.format("DD.MM.YY HH:mm")]);
  	}

  	return data;
  }

	onMount(async () => {
		allSupervisors = await (await fetch("/admin/betreuer.json")).json();
		// showSupervisors(); TODO filter
	});
</script>

<div class="form-inline">
	<div class="form-group mx-sm-3 mb-2">
    <!-- svelte-ignore a11y-autofocus -->
		<input class="form-control" id="supervisorFilter" type="text" autofocus="autofocus" oninput="showSupervisors()" placeholder="Suchen…" />
	</div>
	<div class="mx-sm-3 mb-2">
    <!-- svelte-ignore a11y-invalid-attribute -->
    <a on:click={() => console.log("hi")} href="#">.csv</a>
		<a href="javascript:createCsv(createSupervisorData(), false)">.csv</a>
		<a href="javascript:createXlsx(createSupervisorData(true), false)">.xlsx</a>
	</div>
</div>

<table class="table">
	<thead class="thead-light">
		<tr>
			<th scope="col">Name</th>
			<th scope="col"><!-- Geschlecht --></th>
			<th scope="col">Geburtsdatum</th>
			<th scope="col">JuLeiCa</th>
			<th scope="col">E-Mail</th>
			<th scope="col">Handy</th>
			<th scope="col">Adresse</th>
			<th scope="col">Ort</th>
			<th scope="col">PLZ</th>
			<th scope="col">Führungszeugnis Ausstellung</th>
			<th scope="col">Führungszeugnis Eingesehen</th>
			<th scope="col">Krankenversicherung</th>
			<th scope="col">Tetanus-Impfung</th>
			<th scope="col">Vege&shy;tarier</th>
			<th scope="col">Allergien</th>
			<th scope="col">Unverträglichkeiten</th>
			<th scope="col">Medikamente</th>
			<th scope="col">Besonderheiten</th>
			<th scope="col">Anmeldedatum</th>
		</tr>
	</thead>
	<tbody>
		{#if allSupervisors !== undefined}
			{#each allSupervisors as e}
				<tr>
					<td>{e.vorname} {e.nachname}</td>
					<td>{e.geschlecht === "Male" ? "m" : "w"}</td>
					<td>{moment.utc(e.geburtsdatum).local().format("DD.MM.YYYY")}</td>
					<td>{e.juleica_nummer || ""}</td>
					<td>{e.mail}</td>
					<td>{e.handynummer}</td>
					<td>{e.strasse} {e.hausnummer}</td>
					<td>{e.ort}</td>
					<td>{e.plz}</td>
					<td>{e.fuehrungszeugnis_auststellung && moment.utc(e.fuehrungszeugnis_auststellung).local().format("DD.MM.YYYY") || ""}</td>
					<td>{e.fuehrungszeugnis_eingesehen && moment.utc(e.fuehrungszeugnis_eingesehen).local().format("DD.MM.YYYY") || ""}</td>
					<td>{e.krankenversicherung}</td>
					<td><input type="checkbox" checked={e.tetanus_impfung} disabled /></td>
					<td><input type="checkbox" checked={e.vegetarier} disabled /></td>
					<td>{e.allergien}</td>
					<td>{e.unvertraeglichkeiten}</td>
					<td>{e.medikamente}</td>
					<td>{e.besonderheiten}</td>
					<td>{moment.utc(e.anmeldedatum).local().format("DD.MM.YY HH:mm")}</td>
				</tr>
			{/each}
		{/if}
	</tbody>
</table>

<style>
</style>