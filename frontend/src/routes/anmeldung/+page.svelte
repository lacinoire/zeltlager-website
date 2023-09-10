<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { browser, building } from "$app/environment";
	import { YEAR } from "$lib/utils";
	import Icon from "$lib/Icon.svelte";
	import { mdiDelete } from "@mdi/js";

	let error: string | undefined;
	let isLoading = false;
	let isFull = false;
	let signupForm: HTMLFormElement | undefined;
	let errorMsg: HTMLElement | undefined;
	let curCategory: number = 0;
	let signupFormSaved = false;

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
		// Defaults to name.toLowerCase()
		id?: string;
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

		{ "name": "Zusätzliche Angaben", "id": "zusatz", "fields": [
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

		{ "name": "Überprüfen & Absenden", "id": "ueberpruefen", "fields": [
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

	function saveEntries() {
		const form = {};
		for (const c of CATEGORIES) {
			for (const f of c.fields) {
				const id = f.id ?? f.name.toLowerCase();
				if (id !== "agb" && signupForm[id].value !== "")
					form[id] = signupForm[id].value;
			}
		}
		if (Object.keys(form).length !== 0) {
			localStorage.signupForm = JSON.stringify(form);
			signupFormSaved = true;
		} else {
			localStorage.removeItem("signupForm");
			signupFormSaved = false;
		}
	}

	function loadEntries() {
		if (localStorage.signupForm === undefined) return;
		signupFormSaved = true;
		const form = JSON.parse(localStorage.signupForm);
		for (const c of CATEGORIES) {
			for (const f of c.fields) {
				const id = f.id ?? f.name.toLowerCase();
				if (id in form)
					signupForm[id].value = form[id];
			}
		}
	}

	function clearEntries() {
		localStorage.removeItem("signupForm");
		signupFormSaved = false;
		for (const c of CATEGORIES) {
			for (const f of c.fields) {
				const id = f.id ?? f.name.toLowerCase();
				signupForm[id].value = "";
			}
		}
	}

	onMount(() => {
		loadState();
		loadEntries();

		if (browser) {
			// Set category by location hash
			const loc = location.hash;
			if (loc && loc !== "" && loc !== "#") {
				const id = loc.substring(1);
				for (let i = 0; i < CATEGORIES.length; i++) {
					const catId = CATEGORIES[i].id ?? CATEGORIES[i].name.toLowerCase();
					if (catId === id) {
						curCategory = i;
						break;
					}
				}
			}

			// Remove required classes for firefox on android, it doesn't show any popup there
			const userAgent = navigator.userAgent.toLowerCase();
			if (userAgent.includes("android") && userAgent.includes("firefox") && signupForm) {
				signupForm.querySelectorAll("input").forEach((element) => (element.required = false));
			}
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

<div class="progress-indicator-container">
	<div class="progress-indicator">
		{#each CATEGORIES as category, i}
			<div class="category" class:active={i == curCategory} class:finished={i < curCategory}>
				{#if i > 0}
					<div class="bar"></div>
				{/if}
				<div class="knob label-container">
					<div class="progress-label">
						<a href={`#${category.id ?? category.name.toLowerCase()}`} on:click={() => curCategory = i}>{category.name}</a>
					</div>
				</div>
			</div>
		{/each}
	</div>
</div>

<form
	method="post"
	action="/api/signup-nojs"
	class:is-hidden={isFull}
	on:submit|preventDefault={signup}
	bind:this={signupForm}>

	{#each CATEGORIES as category, i}
		<div class:is-hidden={(i != curCategory && !building && curCategory != CATEGORIES.length - 1) || category.fields.length == 0}>
			<h2 class="title is-4" id="">{category.name}</h2>
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
										on:blur={saveEntries}
										type={field.type ?? "text"} />
								{:else if field.type === "radio"}
									{#each field.variants ?? DEFAULT_VARIANTS as variant}
										<label class="radio">
											<input name={field.id ?? field.name.toLowerCase()} value={variant.id ?? variant.name.toLowerCase()} required type="radio" on:change={saveEntries} />
											<span class="custom-control-indicator" />
											<span class="custom-control-description">{variant.name}</span>
										</label>
									{/each}
								{:else if field.type === "textarea"}
									<textarea
										id={field.id ?? field.name.toLowerCase()}
										name={field.id ?? field.name.toLowerCase()}
										on:blur={saveEntries}
										cols="40"
										rows="1"
										class="textarea"
										aria-describedby={field.help !== undefined ? ((field.id ?? field.name.toLowerCase()) + "HelpBlock") : undefined} />
								{:else if field.type === "checkbox"}
									<label class="checkbox">
										<input name={field.id ?? field.name.toLowerCase()} value="true" required type="checkbox" on:change={saveEntries} />
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
		</div>
	{/each}

	<br>
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
					{#if curCategory != 0 && !building}
						<button class="button is-info" on:click|preventDefault={() => curCategory--}>
							Zurück
						</button>
					{/if}
					{#if curCategory == CATEGORIES.length - 1 || building}
						<button type="submit" class="button is-primary" class:is-loading={isLoading && error === undefined}>
							Zum Zeltlager anmelden
						</button>
					{:else}
						<button class="button is-info" on:click|preventDefault={() => curCategory++}>
							Weiter
						</button>
					{/if}
					{#if signupFormSaved}
						<button class="button reset-button" on:click|preventDefault={clearEntries} title="Formular zurücksetzen">
							<Icon name={mdiDelete} />
						</button>
					{/if}
				</div>
			</div>
		</div>
	</div>
</form>

<style lang="scss">
	.error-msg {
		margin-bottom: 1em;
	}

	h2.title.is-4 {
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
	.reset-button {
		float: right;
	}

	.progress-indicator-container {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	$knob-size: 1em;
	$knob-margin: 0.7em;
	$bar-thickness: 0.2em;
	$bar-margin: calc($knob-margin + $knob-size / 2 - $bar-thickness / 2);
	.progress-indicator {

		display: flex;
		flex-direction: row;

		// Space for the label
		margin-top: 3em;

		.category {
			display: flex;
			align-items: center;
			flex-direction: row;

			.knob {
				background-color: #ddd;
				border-radius: 100%;
				border: 0.15em solid white;
				padding: 0.15em;
				margin: $knob-margin;
				width: $knob-size;
				height: $knob-size;
				box-sizing: border-box;
				background-clip: content-box;
			}

			.bar {
				background-color: #eec73d;
				height: $bar-thickness;
				width: 13em;
				padding: 0;
			}

			.label-container {
				position: relative;
			}

			.progress-label {
				position: absolute;
				transform: translate(-50%, -2em);
				text-align: center;
				width: 15em;
				font-size: 1.2em;

				a {
					color: inherit;
					&:hover {
						color: hsl(229, 53%, 53%);
					}
				}
			}

			&.active {
				.knob {
					border-color: #eec73d;
				}

				.bar {
						background-color: #0eb100;
				}

				.progress-label {
					font-weight: bold;
				}
			}

			&.finished {
				.knob {
					border-color: #0eb100;
				}

				.bar {
						background-color: #0eb100;
				}
			}
		}
	}

	@media screen and (max-width: 1230px) {
		.button {
			width: 100%;
		}

		.progress-indicator-container {
			display: inherit;
		}

		.progress-indicator {
			flex-direction: column;

			.category {
				flex-direction: column;
				align-items: start;

				.bar {
					width: $bar-thickness;
					height: 4em;
					margin-left: $bar-margin;
				}

				.progress-label {
					transform: translate(2em, -0.5em);
					text-align: left;
				}
			}
		}
	}
</style>
