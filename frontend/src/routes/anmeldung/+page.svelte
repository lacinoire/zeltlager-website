<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { YEAR } from "$lib/utils";

	let error: string | undefined;
	let isLoading = false;
	let isFull = false;
	let signupForm: HTMLFormElement | undefined;
	let errorMsg: HTMLElement | undefined;

	interface Variant {
		// Defaults to name.toLowerCase()
		id?: string;
		name: string;
	}

	interface Field {
		name: string;
		// Defaults to name.toLowerCase()
		id?: string;
		defaultValue?: string;
		// Defaults to name
		placeholder?: string;
		autocomplete?: string;
		inputmode?: string;
		// Defaults to true
		required?: bool;
		// Defaults to text
		type?: string;
		help?: string;
		// For type=radio, defaults to DEFAULT_VARIANTS
		variants?: Variant[];
	}

	interface Category {
		name: string;
		fields: Field[];
	}

	const DEFAULT_VARIANTS: Variant[] = [
		{ "id": "true", "name": "Ja" },
		{ "id": "false", "name": "Nein" },
	];

	const CATEGORIES: Category[] = [
		{ "name": "Kind", "fields": [
			{ "name": "Vorname", "placeholder": "Vorname des Kindes", "autocomplete": "given-name" },
			{ "name": "Nachname", "placeholder": "Nachname des Kindes", "autocomplete": "family-name" },
			{ "name": "Geburtsdatum", "placeholder": "TT.MM.JJJJ", "autocomplete": "bday" },
			{ "name": "Geschlecht", "type": "radio", "variants": [ { "id": "m", "name": "Männlich" }, { "id": "w", "name": "Weiblich" } ] },
			{ "name": "Schwimmer", "type": "radio", "variants": [ { "id": "true", "name": "Schwimmer" }, { "id": "false", "name": "Nichtschwimmer" } ] },
			{ "name": "Vegetarier", "type": "radio" },
			{ "id": "tetanus_impfung", "name": "Tetanusimpfung", "type": "radio" },
			{ "name": "Krankenversicherung", "type": "radio", "variants": [ { "name": "Gesetzlich" }, { "name": "Privat" }, { "name": "Anderes" } ] },
		] },

		{ "name": "Adresse", "fields": [
			{ "name": "Land", "defaultValue": "Deutschland", "autocomplete": "country-name" },
			{ "id": "strasse", "name": "Straße" },
			{ "name": "Hausnummer" },
			{ "id": "plz", "name": "Postleitzahl", "placeholder": "PLZ", "autocomplete": "postal-code", "inputmode": "numeric" },
			{ "name": "Ort" },
		] },

		{ "name": "Erziehungsberechtigte", "fields": [
			{ "id": "eltern_name", "name": "Name eines Erziehungsberechtigten", "placeholder": "Name eines Erziehungsberechtigten des Kindes", "autocomplete": "name" },
			{ "id": "eltern_mail", "name": "E-Mailadresse des Erziehungsberechtigten", "placeholder": "E-Mail des Erziehungsberechtigten", "autocomplete": "email", "inputmode": "email", "type": "email" },
			{ "id": "eltern_handynummer", "name": "Handynummer des Erziehungsberechtigten (für Notfälle)", "placeholder": "Handynummer des Erziehungsberechtigten", "autocomplete": "tel", "inputmode": "tel" },
		] },

		{ "name": "Zusätzliche Angaben", "fields": [
			{ "name": "Allergien", "type": "textarea", "help": "z.B. Haselnussallergie", "required": false },
			{ "id": "unvertraeglichkeiten", "name": "Lebens&shy;mittel&shy;unver&shy;träglichkeiten", "type": "textarea", "help": "z.B. Laktoseintoleranz, kein Schweinefleisch", "required": false },
			{ "name": "Medikamente", "type": "textarea", "help": "z.B. Asthmaspray; Methylphenidat, 10 mg", "required": false },
			{ "name": "Besonderheiten", "type": "textarea", "help": "Krankheiten, Eigenheiten, etc.", "required": false },
			{ "id": "agb", "type": "checkbox", "name": 'Ich habe die <a href="/agb" target="_blank">\
							Allgemeine Geschäftsbedingungen\
						</a>\
						und die\
						<a href="/datenschutz" target="_blank">Datenschutzbestimmungen</a> gelesen und\
						akzeptiere sie.' },
		] },
	];

	async function loadState() {
		isFull = false;
		let response: Response;
		try {
			response = await fetch("/api/signup-state");
		} catch (e) {
			console.error("Failed to make signup state web request", e);
			error = "Verbindung fehlgeschlagen. Ist das Internet erreichbar?";
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read signup state response", e);
			error = "Verbindung abgebrochen";
			return;
		}
		try {
			const resp = JSON.parse(respText);
			isFull = resp.isFull;
		} catch (e) {
			console.error("Failed to convert signup state request to json", e);
			error = respText;
			return;
		}
	}

	function setError(msg: string) {
		error = msg;
		errorMsg?.scrollIntoView({ behavior: "smooth" });
	}

	async function signup() {
		if (isLoading && error === undefined) return;
		error = undefined;
		isLoading = true;

		let response: Response;
		try {
			response = await fetch("/api/signup", {
				method: "POST",
				headers: {
					"Content-Type": "application/x-www-form-urlencoded; charset=utf-8",
				},
				body: new URLSearchParams(new FormData(signupForm) as any),
			});
		} catch (e) {
			console.error("Failed to make signup web request", e);
			setError("Verbindung fehlgeschlagen. Ist das Internet erreichbar?");
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read signup response", e);
			setError("Verbindung abgebrochen");
			return;
		}
		try {
			const resp = JSON.parse(respText);
			if (resp.error !== null) {
				setError(resp.error);
			} else {
				// Signup successful
				goto("/anmeldung-erfolgreich");
				return;
			}
		} catch (e) {
			console.error("Failed to convert signup request to json", e);
			setError(respText);
			return;
		}
		// Refetch status
		await loadState();
		isLoading = false;
	}

	function fillTestData() {
		if (signupForm === undefined) return;
		signupForm.vorname.value = "a";
		signupForm.nachname.value = "b";
		signupForm.geburtsdatum.value = "1.1.2010";
		signupForm.geschlecht.value = "w";
		signupForm.schwimmer.value = "true";
		signupForm.vegetarier.value = "false";
		signupForm.tetanus_impfung.value = "true";
		signupForm.krankenversicherung.value = "gesetzlich";
		signupForm.eltern_name.value = "c";
		signupForm.eltern_mail.value = "a@b";
		signupForm.eltern_handynummer.value = "d";
		signupForm.strasse.value = "e";
		signupForm.hausnummer.value = "1";
		signupForm.plz.value = "80000";
		signupForm.ort.value = "f";
	}

	function shortcut(e: KeyboardEvent) {
		// Press Alt+Escape in Nachname to fill in test data
		if (e.altKey && e.key === "Escape") fillTestData();
	}

	onMount(() => {
		loadState();

		// Remove required classes for firefox on android, it doesn't show any popup there
		const userAgent = browser ? navigator.userAgent.toLowerCase() : "";
		if (userAgent.includes("android") && userAgent.includes("firefox") && signupForm) {
			signupForm.querySelectorAll("input").forEach((element) => (element.required = false));
		}
	});
</script>

<svelte:head>
	<title>Anmeldung – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title">Anmeldung für das Zeltlager {YEAR}</h1>

<div bind:this={errorMsg} class="error-msg">
	{#if error !== undefined}
		<article class="message is-danger">
			<div class="message-body">
				{error}
			</div>
		</article>
	{/if}
</div>

{#if isFull}
	<article class="message is-info">
		<div class="message-body">Das Zeltlager für dieses Jahr ist leider schon voll.</div>
	</article>

	<p>
		Unsere <a href="/agb">AGB</a> und <a href="/datenschutz">Datenschutzbestimmungen</a>.
	</p>
{/if}

<form
	method="post"
	action="/api/signup-nojs"
	class:is-hidden={isFull}
	on:submit|preventDefault={signup}
	bind:this={signupForm}>

	{#each CATEGORIES as category}
		<h2 class="title is-4">{category.name}</h2>
		{#each category.fields as field}
			<div class="field is-horizontal" class:required={field.required ?? true}>
				<div class="field-label">
					{#if field.type !== "checkbox"}
						<label for={field.id ?? field.name.toLowerCase()} class="label">{@html field.name}</label>
					{/if}
				</div>
				<div class="field-body">
					<div class="field">
						<div class="control">
							{#if field.type === undefined || field.type === "text" || field.type === "email"}
								<input
									id={field.id ?? field.name.toLowerCase()}
									name={field.id ?? field.name.toLowerCase()}
									placeholder={field.placeholder ?? field.name}
									required={field.required ?? true}
									class="input"
									autocomplete={field.autocomplete ?? false}
									value={field.defaultValue ?? ""}
									inputmode={field.inputmode ?? ""}
									on:keydown={field.name === "Nachname" ? shortcut : undefined}
									type={field.type ?? "text"} />
							{:else if field.type === "radio"}
								{#each field.variants ?? DEFAULT_VARIANTS as variant}
									<label class="radio">
										<input name={field.id ?? field.name.toLowerCase()} value={variant.id ?? variant.name.toLowerCase()} required type="radio" />
										<span class="custom-control-indicator" />
										<span class="custom-control-description">{variant.name}</span>
									</label>
								{/each}
							{:else if field.type === "textarea"}
								<textarea
									id={field.id ?? field.name.toLowerCase()}
									name={field.id ?? field.name.toLowerCase()}
									cols="40"
									rows="1"
									class="textarea"
									aria-describedby={field.help !== undefined ? ((field.id ?? field.name.toLowerCase()) + "HelpBlock") : undefined} />
							{:else if field.type === "checkbox"}
								<label class="checkbox">
									<input name={field.id ?? field.name.toLowerCase()} value="true" required type="checkbox" />
									<span class="custom-control-indicator" />
									<span class="custom-control-description">{@html field.name}</span>
								</label>
							{/if}
						</div>
						{#if field.help !== undefined || field.required === false}
							<p id={(field.id ?? field.name.toLowerCase()) + "HelpBlock"} class="help">
								{field.help ?? ""}
								{#if field.required === false}
									<p class="optional">Optional</p>
								{/if}
							</p>
						{/if}
					</div>
				</div>
			</div>
		{/each}
	{/each}


	<div class="field is-horizontal required">
		<div class="field-label" />
		<div class="field-body">
			<div class="required"><span class="label" style="display: inline;" />Pflichtfeld</div>
		</div>
	</div>

	<div class="field is-horizontal">
		<div class="field-label" />
		<div class="field-body">
			<div class="field">
				<div class="control">
					<button type="submit" class="button is-primary" class:is-loading={isLoading && error === undefined}>
						Zum Zeltlager anmelden
					</button>
				</div>
			</div>
		</div>
	</div>
</form>

<style>
	.error-msg {
		margin-bottom: 1em;
	}

	.title.is-4 {
		margin-top: 3em;
		margin-bottom: 1.2em;
	}

	form > .field:not(:last-child) {
		margin-bottom: 1.5em;
	}

	.button {
		margin-top: 2em;
	}

	.optional {
		float: right;
		font-style: italic;
	}

	@media screen and (max-width: 768px) {
		.button {
			width: 100%;
		}
	}
</style>
