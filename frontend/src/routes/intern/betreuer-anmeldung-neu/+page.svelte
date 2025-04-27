<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import moment from "moment";
	import { inferPlace } from "$lib/utils";
	import PagedForm from "$lib/PagedForm.svelte";
	import type { Category } from "$lib/PagedForm.svelte";

	let form: HTMLFormElement | undefined = $state();
	let loading = $state(false);

	const CATEGORIES: Category[] = [
		{
			name: "Persönliche Daten",
			fields: [
				{ name: "Vorname", placeholder: "Vorname", autocomplete: "given-name" },
				{
					name: "Nachname",
					placeholder: "Nachname",
					autocomplete: "family-name",
					keydown: shortcut,
				},
				{ name: "Geburtsdatum", placeholder: "TT.MM.JJJJ", autocomplete: "bday" },
				{
					name: "Geschlecht",
					type: "radio",
					variants: [
						{ id: "m", name: "Männlich" },
						{ id: "w", name: "Weiblich" },
					],
				},
				{
					id: "mail",
					name: "E-Mailadresse",
					placeholder: "E-Mailadresse",
					autocomplete: "email",
					inputmode: "email",
					type: "email",
				},
				{ name: "Handynummer", autocomplete: "tel", inputmode: "tel" },
			],
		},

		{
			name: "Zeltlager-relevantes",
			fields: [
				{ id: "tetanus_impfung", name: "Tetanusimpfung", type: "radio" },
				{
					name: "Krankenversicherung",
					type: "radio",
					variants: [{ name: "Gesetzlich" }, { name: "Privat" }, { name: "Anderes" }],
				},
				{
					id: "juleica_nummer",
					name: "Juleica Nummer",
					placeholder: "Juleicanummer",
					required: false,
				},
				{
					id: "juleica_gueltig_bis",
					name: "Juleica gültig bis",
					placeholder: "dd.mm.yyyy",
					required: false,
				},
				{
					id: "fuehrungszeugnis_ausstellung",
					name: "Austellungsdatum Erweitertes Führungszeugnis",
					placeholder: "TT.MM.JJJJ",
					required: false,
				},
			],
		},

		{
			name: "Adresse",
			fields: [
				{ name: "Land", defaultValue: "Deutschland", autocomplete: "country-name" },
				{ id: "strasse", name: "Straße" },
				{ name: "Hausnummer" },
				{
					id: "plz",
					name: "Postleitzahl",
					placeholder: "PLZ",
					autocomplete: "postal-code",
					inputmode: "numeric",
					focusout: inferPlaceWrapper,
				},
				{ name: "Ort" },
			],
		},

		{
			name: "Zusätzliche Angaben",
			id: "zusatz",
			fields: [
				{
					id: "vegetarier",
					name: "Ich esse nur vegetarisch",
					type: "radio",
				},
				{
					id: "unvertraeglichkeiten",
					name: "Lebens&shy;mittel&shy;unver&shy;träglichkeiten/-allergien",
					type: "textarea",
					help: "z.B. Haselnussallergie, Laktoseintoleranz, kein Schweinefleisch, vegan",
					required: false,
				},
				{
					id: "allergien",
					name: "Sonstige Allergien",
					type: "textarea",
					help: "z.B. Tierhaare, Pollen",
					required: false,
				},
				{
					id: "krankheiten",
					name: "Eigenheiten/Krankheiten",
					type: "textarea",
					help: "z.B. ADHS, etc.",
					required: false,
				},
				{
					name: "Medikamente",
					type: "textarea",
					help: "z.B. Lotemax 5mg, morgens und abends<br>Asthmaspray; Methylphenidat, 10 mg, bei Bedarf",
					required: false,
				},
				{
					id: "kommentar",
					name: "Sonstige Kommentare",
					type: "textarea",
					required: false,
				},
				{
					id: "selbsterklaerung",
					type: "checkbox",
					name: 'Ich habe die <a\
						href="/intern/selbstverpflichtung" target="_blank">\
							Selbstverpflichtungserklärung zur Prävention von sexualisierter Gewalt\
						</a>\
						sowie den\
						<a href="/intern/selbstverpflichtung-anhang" target="_blank">Anhang</a> gelesen\
						und verpflichte mich, mich daran zu halten.',
				},
				{
					id: "agb",
					type: "checkbox",
					name: 'Ich habe die <a href="/agb" target="_blank">\
							Allgemeine Geschäftsbedingungen\
						</a>\
						und die\
						<a href="/datenschutz" target="_blank">Datenschutzbestimmungen</a> gelesen und\
						akzeptiere sie.',
				},
			],
		},

		{ name: "Überprüfen & Absenden", id: "ueberpruefen", fields: [] },
	];

	async function signup() {
		let response: Response;
		try {
			response = await fetch("/api/signup-supervisor", {
				method: "POST",
				headers: {
					"Content-Type": "application/x-www-form-urlencoded; charset=utf-8",
				},
				body: new URLSearchParams(new FormData(form.form) as any),
			});
		} catch (e) {
			console.error("Failed to make signup web request", e);
			form?.setError("Verbindung fehlgeschlagen. Ist das Internet erreichbar?");
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read signup response", e);
			form?.setError("Verbindung abgebrochen");
			return;
		}
		try {
			const resp = JSON.parse(respText);
			if (resp.error !== null) {
				form?.setErrorMsg(resp.error);
			} else {
				// Signup successful
				form?.clearEntries();
				goto("/intern/betreuer-anmeldung-erfolgreich");
				return;
			}
		} catch (e) {
			console.error("Failed to convert signup request to json", e);
			form?.setError(respText);
			return;
		}
	}

	function fillTestData() {
		if (form === undefined) return;
		const f = form.form;
		f.vorname.value = "a";
		f.nachname.value = "b";
		f.geburtsdatum.value = "1.1.2000";
		f.geschlecht.value = "w";
		f.vegetarier.value = "true";
		f.tetanus_impfung.value = "true";
		f.krankenversicherung.value = "gesetzlich";
		f.mail.value = "a@b";
		f.handynummer.value = "d";
		f.land.value = "Deutschland";
		f.strasse.value = "e";
		f.hausnummer.value = "1";
		f.plz.value = "80000";
		f.ort.value = "f";
	}

	async function fillTokenData() {
		if (form === undefined) return;
		const urlParams = new URLSearchParams(window.location.search);
		let token = urlParams.get("token");
		if (token === null) return;

		loading = true;
		let response: Response;
		try {
			response = await fetch("/api/get-supervisor-data", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({ token }),
			});
		} catch (e) {
			console.error("Failed to get data web request", e);
			form?.setError("Verbindung fehlgeschlagen. Ist das Internet erreichbar?");
			loading = false;
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read data response", e);
			form?.setError("Verbindung abgebrochen");
			loading = false;
			return;
		}
		let data;
		try {
			data = JSON.parse(respText);
			if (data.error !== undefined) {
				form?.setError(data.error);
				loading = false;
				return;
			} else {
				// Got data successfully
			}
		} catch (e) {
			console.error("Failed to convert signup request to json", e);
			form?.setError(respText);
			loading = false;
			return;
		}

		const f = form.form;
		f.vorname.value = data.vorname;
		f.nachname.value = data.nachname;
		f.geburtsdatum.value = moment.utc(data.geburtsdatum).format("DD.MM.YYYY");
		f.geschlecht.value = data.geschlecht === "Male" ? "m" : "w";
		f.vegetarier.value = data.vegetarier;
		f.juleica_nummer.value = data.juleica_nummer;
		f.mail.value = data.mail;
		f.handynummer.value = data.handynummer;
		f.strasse.value = data.strasse;
		f.hausnummer.value = data.hausnummer;
		f.ort.value = data.ort;
		f.plz.value = data.plz;
		f.kommentar.value = data.kommentar;
		f.fuehrungszeugnis_ausstellung.value = data.fuehrungszeugnis_ausstellung ? moment.utc(data.fuehrungszeugnis_ausstellung).format("DD.MM.YYYY") : "";
		f.allergien.value = data.allergien;
		f.unvertraeglichkeiten.value = data.unvertraeglichkeiten;
		f.medikamente.value = data.medikamente;
		f.krankenversicherung.value = data.krankenversicherung;
		f.tetanus_impfung.value = data.tetanus_impfung;
		f.land.value = data.land;
		f.krankheiten.value = data.krankheiten;
		f.juleica_gueltig_bis.value = data.juleica_gueltig_bis ? moment.utc(data.juleica_gueltig_bis).format("DD.MM.YYYY") : "";

		loading = false;
	}

	function inferPlaceWrapper() {
		inferPlace(form.form);
	}

	function shortcut(e: KeyboardEvent) {
		// Press Alt+Escape in Nachname to fill in test data
		if (e.altKey && e.key === "Escape") fillTestData();
	}

	onMount(fillTokenData);
</script>

<svelte:head>
	<title>Betreueranmeldung – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title">Betreueranmeldung für das Zeltlager
	{#if loading}
		<button class="button is-loading">Loading</button>
	{/if}
</h1>

<PagedForm
	bind:this={form}
	name="signupSupervisorForm"
	categories={CATEGORIES}
	submitText="Als Betreuer anmelden"
	nojs_submit_url="/api/signup-supervisor-nojs"
	on:submit={signup} />

<style>
	.title .button.is-loading {
		border: none;
	}
</style>
