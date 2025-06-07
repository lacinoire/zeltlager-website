<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { goto } from "$app/navigation";
	import PagedForm from "$lib/PagedForm.svelte";
	import type { Category } from "$lib/PagedForm.svelte";

	let form: HTMLFormElement | undefined = $state();
	let isLoading = false;

	const CATEGORIES: Category[] = [
		{
			name: "",
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
					id: "grund",
					name: "Deshalb will ich ins Zeltlager fahren",
					type: "textarea",
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
				{
					id: "kommentar",
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
	];

	async function signup() {
		let response: Response;
		try {
			response = await fetch("/api/presignup-supervisor", {
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
				goto("/betreuer-anmeldung-erfolgreich");
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
		f.mail.value = "a@b";
		f.handynummer.value = "d";
		f.grund.value = "e";
	}

	function shortcut(e: KeyboardEvent) {
		// Press Alt+Escape in Nachname to fill in test data
		if (e.altKey && e.key === "Escape") fillTestData();
	}
</script>

<svelte:head>
	<title>Für Betreuer – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title">Anmeldung als neue:r Betreuer:in</h1>

<div class="content">
<p>
Vor dem Lager muss jede:r Neu-Betreuer:in einen Juleica-Kurs gemacht haben. Informationen zu Terminen, der Anmeldung etc. findest du auf <a target="_blank" href="https://www.msj.de/bildung/juleica/">www.msj.de/bildung/juleica/</a>.<br />
Egal ob du den Kurs schon gemacht hast oder nicht, wenn du als Betreuer im Zeltlager mitfahren willst, melde dich hier an, wir schreiben dich dann an.
</p>

<p>
<strong>Wenn du schon in der Betreuer-Gruppe bist, musst du dich hier nicht anmelden, melde dich stattdessen mit dem Link in der Gruppe an.</strong>
</p>
</div>

<PagedForm
	bind:this={form}
	name="signupSupervisorForm"
	categories={CATEGORIES}
	submitText="Als Betreuer anmelden"
	nojs_submit_url="/api/presignup-supervisor-nojs"
	on:submit={signup} />

<style>
</style>
