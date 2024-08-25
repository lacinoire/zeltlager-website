<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { YEAR, inferPlace } from "$lib/utils";
	import PagedForm from "$lib/PagedForm.svelte";
	import type { Category } from "$lib/PagedForm.svelte";

	let isFull = false;
	let form: PagedForm | undefined;

	const CATEGORIES: Category[] = [
		{
			name: "Kind",
			fields: [
				{ name: "Vorname", placeholder: "Vorname des Kindes", autocomplete: "given-name" },
				{
					name: "Nachname",
					placeholder: "Nachname des Kindes",
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
					name: "Schwimmer",
					type: "radio",
					variants: [
						{ id: "true", name: "Schwimmer" },
						{ id: "false", name: "Nichtschwimmer" },
					],
				},
				{ name: "Vegetarier", type: "radio" },
				{ id: "tetanus_impfung", name: "Tetanusimpfung", type: "radio" },
				{
					name: "Krankenversicherung",
					type: "radio",
					variants: [{ name: "Gesetzlich" }, { name: "Privat" }, { name: "Anderes" }],
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
			name: "Erziehungsberechtigte",
			fields: [
				{
					id: "eltern_name",
					name: "Name eines Erziehungsberechtigten",
					placeholder: "Name eines Erziehungsberechtigten des Kindes",
					autocomplete: "name",
				},
				{
					id: "eltern_mail",
					name: "E-Mailadresse des Erziehungsberechtigten",
					placeholder: "E-Mail des Erziehungsberechtigten",
					autocomplete: "email",
					inputmode: "email",
					type: "email",
				},
				{
					id: "eltern_handynummer",
					name: "Handynummer des Erziehungsberechtigten (für Notfälle)",
					placeholder: "Handynummer des Erziehungsberechtigten",
					autocomplete: "tel",
					inputmode: "tel",
				},
			],
		},

		{
			name: "Zusätzliche Angaben",
			id: "zusatz",
			fields: [
				{
					id: "unvertraeglichkeiten",
					name: "Lebens&shy;mittel&shy;unver&shy;träglichkeiten/-allergien",
					type: "textarea",
					help: "z.B. Haselnussallergie, Laktoseintoleranz, kein Schweinefleisch",
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
					name: "Medikamente",
					type: "textarea",
					help: "z.B. Lotemax 5mg, morgens und abends<br>Asthmaspray; Methylphenidat, 10 mg, bei Bedarf",
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
					id: "besonderheiten",
					name: "Sonstige Kommentare",
					type: "textarea",
					required: false,
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

	async function loadState() {
		isFull = false;
		let response: Response;
		try {
			response = await fetch("/api/signup-state");
		} catch (e) {
			console.error("Failed to make signup state web request", e);
			form.setError("Verbindung fehlgeschlagen. Ist das Internet erreichbar?");
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read signup state response", e);
			form.setError("Verbindung abgebrochen");
			return;
		}
		try {
			const resp = JSON.parse(respText);
			isFull = resp.isFull;
		} catch (e) {
			console.error("Failed to convert signup state request to json", e);
			form.setError(respText);
			return;
		}
	}

	async function signup() {
		let response: Response;
		try {
			response = await fetch("/api/signup", {
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
				// TODO add option to not clear (non-child) entries
				form?.clearEntries();
				goto("/anmeldung-erfolgreich");
				return;
			}
		} catch (e) {
			console.error("Failed to convert signup request to json", e);
			form?.setError(respText);
			return;
		}
		// Refetch status
		await loadState();
	}

	function fillTestData() {
		if (form === undefined) return;
		const f = form.form;
		f.vorname.value = "a";
		f.nachname.value = "b";
		f.geburtsdatum.value = "1.1.2010";
		f.geschlecht.value = "w";
		f.schwimmer.value = "true";
		f.vegetarier.value = "false";
		f.tetanus_impfung.value = "true";
		f.krankenversicherung.value = "gesetzlich";
		f.eltern_name.value = "c";
		f.eltern_mail.value = "a@b";
		f.eltern_handynummer.value = "d";
		f.land.value = "Deutschland";
		f.strasse.value = "e";
		f.hausnummer.value = "1";
		f.plz.value = "80000";
		f.ort.value = "f";
	}

	function shortcut(e: KeyboardEvent) {
		// Press Alt+Escape in Nachname to fill in test data
		if (e.altKey && e.key === "Escape") fillTestData();
	}

	function inferPlaceWrapper() {
		inferPlace(form.form);
	}

	onMount(() => {
		loadState();
	});
</script>

<svelte:head>
	<title>Anmeldung – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title">Anmeldung für das Zeltlager {YEAR}</h1>

{#if isFull}
	<article class="message is-info">
		<div class="message-body">Das Zeltlager für dieses Jahr ist leider schon voll.</div>
	</article>

	<p>
		Unsere <a href="/agb">AGB</a> und <a href="/datenschutz">Datenschutzbestimmungen</a>.
	</p>
{/if}

<div class:is-hidden={isFull}>
	<PagedForm
		bind:this={form}
		name="signupForm"
		categories={CATEGORIES}
		submitText="Zum Zeltlager anmelden"
		nojs_submit_url="/api/signup-nojs"
		on:submit={signup} />
</div>

<style>
</style>
