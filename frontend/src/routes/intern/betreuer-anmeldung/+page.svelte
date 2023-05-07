<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";

	let error: string | undefined;
	let signupForm: HTMLFormElement | undefined;
	let errorMsg: HTMLElement | undefined;

	async function loadState() {
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
		error = undefined;
		let response: Response;
		try {
			response = await fetch("/api/signup-supervisor", {
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
				goto("/intern/betreuer-anmeldung-erfolgreich");
				return;
			}
		} catch (e) {
			console.error("Failed to convert signup request to json", e);
			setError(respText);
			return;
		}
		// Refetch status
		await loadState();
	}

	function fillTestData() {
		if (signupForm === undefined) return;
		signupForm.vorname.value = "a";
		signupForm.nachname.value = "b";
		signupForm.geburtsdatum.value = "1.1.2000";
		signupForm.geschlecht.value = "w";
		signupForm.vegetarier.value = "false";
		signupForm.tetanus_impfung.value = "true";
		signupForm.krankenversicherung.value = "gesetzlich";
		signupForm.mail.value = "a@b";
		signupForm.handynummer.value = "d";
		signupForm.strasse.value = "e";
		signupForm.hausnummer.value = "1";
		signupForm.plz.value = "80000";
		signupForm.ort.value = "f";
	}

	function shortcut(e: KeyboardEvent) {
		// Press Alt+Escape in Nachname to fill in test data
		if (e.altKey && e.key === "Escape") fillTestData();
	}

	onMount(loadState);
</script>

<svelte:head>
	<title>Betreueranmeldung – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title">Betreueranmeldung für das Zeltlager</h1>

<div bind:this={errorMsg} class="error-msg">
	{#if error !== undefined}
		<article class="message is-danger">
			<div class="message-body">
				{error}
			</div>
		</article>
	{/if}
</div>

<form
	method="post"
	action="/api/signup-supervisor-nojs"
	on:submit|preventDefault={signup}
	bind:this={signupForm}>
	<div class="field is-horizontal required">
		<div class="field-label">
			<label for="vorname" class="label">Vorname</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="vorname"
						name="vorname"
						placeholder="Vorname"
						required
						class="input"
						autocomplete="given-name"
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<label for="nachname" class="label">Nachname</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="nachname"
						name="nachname"
						placeholder="Nachname"
						class="input"
						on:keydown={shortcut}
						required
						autocomplete="family-name"
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<label for="geburtsdatum" class="label">Geburtsdatum</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="geburtsdatum"
						name="geburtsdatum"
						placeholder="TT.MM.JJJJ"
						class="input"
						required
						autocomplete="bday"
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<span class="label">Geschlecht</span>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<label class="radio">
						<input name="geschlecht" value="m" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Männlich</span>
					</label>
					<label class="radio">
						<input name="geschlecht" value="w" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Weiblich</span>
					</label>
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<span class="label">Vegetarier</span>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<label class="radio">
						<input name="vegetarier" value="true" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Ja</span>
					</label>
					<label class="radio">
						<input name="vegetarier" value="false" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Nein</span>
					</label>
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<span class="label">Tetanusimpfung</span>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<label class="radio">
						<input name="tetanus_impfung" value="true" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Ja</span>
					</label>
					<label class="radio">
						<input name="tetanus_impfung" value="false" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Nein</span>
					</label>
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<span class="label">Krankenversicherung</span>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<label class="radio">
						<input
							name="krankenversicherung"
							value="gesetzlich"
							required
							type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Gesetzlich</span>
					</label>
					<label class="radio">
						<input name="krankenversicherung" value="privat" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Privat</span>
					</label>
					<label class="radio">
						<input name="krankenversicherung" value="anderes" required type="radio" />
						<span class="custom-control-indicator" />
						<span class="custom-control-description">Anderes</span>
					</label>
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal">
		<div class="field-label">
			<label for="juleica_nummer" class="label">Juleica Nummer</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="juleica_nummer"
						name="juleica_nummer"
						placeholder="Juleicanummer"
						class="input"
						inputmode="numeric"
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<label for="mail" class="label">E-Mailadresse</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="mail"
						name="mail"
						placeholder="E-Mailadresse"
						class="input"
						required
						autocomplete="email"
						inputmode="email"
						type="email" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<label for="handynummer" class="label">Handynummer</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="handynummer"
						name="handynummer"
						placeholder="Handynummer"
						class="input"
						required
						autocomplete="tel"
						inputmode="tel"
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label label">
			<label for="strasse">Straße</label>,
			<label for="hausnummer">Hausnummer</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="strasse"
						name="strasse"
						placeholder="Straße"
						class="input"
						required
						type="text" />
				</div>
			</div>
			<div class="field is-narrow">
				<div class="ontrol">
					<input
						id="hausnummer"
						name="hausnummer"
						placeholder="Hausnummer"
						class="input"
						required
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label label">
			<label for="plz">Postleitzahl</label>,
			<label for="ort">Ort</label>
		</div>
		<div class="field-body">
			<div class="field is-narrow">
				<div class="control">
					<input
						id="plz"
						name="plz"
						placeholder="PLZ"
						class="input"
						required
						autocomplete="postal-code"
						inputmode="numeric"
						type="text" />
				</div>
			</div>
			<div class="field">
				<div class="control">
					<input
						id="ort"
						name="ort"
						placeholder="Ort"
						class="input"
						required
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal">
		<div class="field-label">
			<label for="fuehrungszeugnis_auststellung" class="label"
				>Austellungsdatum Erweitertes Führungszeugnis</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<input
						id="fuehrungszeugnis_auststellung"
						name="fuehrungszeugnis_auststellung"
						placeholder="TT.MM.JJJJ"
						class="input"
						type="text" />
				</div>
			</div>
		</div>
	</div>
	<div class="field is-horizontal">
		<div class="field-label">
			<label for="allergien" class="label">Allergien</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<textarea
						id="allergien"
						name="allergien"
						cols="40"
						rows="1"
						class="textarea"
						aria-describedby="allergienHelpBlock" />
				</div>
				<p id="allergienHelpBlock" class="help">z.B. Haselnussallergie</p>
			</div>
		</div>
	</div>
	<div class="field is-horizontal">
		<div class="field-label">
			<label for="unvertraeglichkeiten" class="label">
				Lebens&shy;mittel&shy;unver&shy;träglichkeiten
			</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<textarea
						id="unvertraeglichkeiten"
						name="unvertraeglichkeiten"
						cols="40"
						rows="1"
						class="textarea"
						aria-describedby="unvertraeglichkeitenHelpBlock" />
				</div>
				<p id="unvertraeglichkeitenHelpBlock" class="help">
					z.B. Laktoseintoleranz, kein Schweinefleisch
				</p>
			</div>
		</div>
	</div>
	<div class="field is-horizontal">
		<div class="field-label">
			<label for="medikamente" class="label">Medikamente</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<textarea
						id="medikamente"
						name="medikamente"
						cols="40"
						rows="1"
						class="textarea"
						aria-describedby="medikamenteHelpBlock" />
				</div>
				<p id="medikamenteHelpBlock" class="help">
					z.B. Asthmaspray; Methylphenidat, 10 mg
				</p>
			</div>
		</div>
	</div>
	<div class="field is-horizontal">
		<div class="field-label">
			<label for="besonderheiten" class="label">Besonderheiten</label>
		</div>
		<div class="field-body">
			<div class="field">
				<div class="control">
					<textarea
						id="besonderheiten"
						name="besonderheiten"
						cols="40"
						rows="2"
						class="textarea"
						aria-describedby="besonderheitenHelpBlock" />
				</div>
				<p id="besonderheitenHelpBlock" class="help">Krankheiten, Eigenheiten, etc.</p>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<span class="label"
				>Selbstverpflichtungserklärung zur Prävention von sexualisierter Gewalt</span>
		</div>
		<div class="field-body">
			<div class="control">
				<label class="checkbox">
					<input name="selbsterklaerung" value="true" required type="checkbox" />
					<span class="custom-control-indicator" />
					<span class="custom-control-description">
						Ich habe die <a href="/intern/selbstverpflichtung" target="_blank"
							>Selbstverpflichtungserklärung zur Prävention von sexualisierter Gewalt</a>
						sowie den
						<a href="/intern/selbstverpflichtung-anhang" target="_blank">Anhang</a> gelesen
						und verpflichte mich, mich daran zu halten.
					</span>
				</label>
			</div>
		</div>
	</div>
	<div class="field is-horizontal required">
		<div class="field-label">
			<span class="label">Allgemeine Geschäftsbedingungen und Datenschutzbestimmungen</span>
		</div>
		<div class="field-body">
			<div class="control">
				<label class="checkbox">
					<input name="agb" value="true" required type="checkbox" />
					<span class="custom-control-indicator" />
					<span class="custom-control-description">
						Ich habe die <a href="/agb" target="_blank">AGB</a> und die
						<a href="/datenschutz" target="_blank">Datenschutzbestimmungen</a> gelesen und
						akzeptiere sie.
					</span>
				</label>
			</div>
		</div>
	</div>
	<div class="field is-horizontal">
		<div class="field-label" />
		<div class="field-body">
			<div class="field">
				<div class="control">
					<button type="submit" class="button is-link">Anmelden</button>
				</div>
			</div>
		</div>
	</div>
</form>

<style>
	.error-msg {
		margin-bottom: 1em;
	}
</style>
