<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { goto } from "$app/navigation";
	import { updateFormRequired } from "$lib/utils";

	let form: HTMLFormElement | undefined;
	let error: string | undefined;
	let isLoading = false;

	async function handleSubmit() {
		// Skip if there is a submit in progress
		if (isLoading && error === undefined) return;
		error = undefined;
		isLoading = true;

		let response: Response;
		try {
			response = await fetch("/api/resignup-supervisor", {
				method: "POST",
				headers: {
					"Content-Type": "application/x-www-form-urlencoded; charset=utf-8",
				},
				body: new URLSearchParams(new FormData(form) as any),
			});
		} catch (e) {
			console.error("Failed to make mail web request", e);
			error = "Verbindung fehlgeschlagen. Ist das Internet erreichbar?";
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read mail response", e);
			error = "Verbindung abgebrochen";
			return;
		}
		try {
			const resp = JSON.parse(respText);
			if (resp.error !== null) {
				error = resp.error;
			} else {
				// Successful
				goto("/intern/betreuer-anmeldung-mail");
				return;
			}
		} catch (e) {
			console.error("Failed to convert mail request to json", e);
			error = respText;
			return;
		}

		isLoading = false;
	}

	onMount(() => {
		if (browser)
			updateFormRequired(form);
	});
</script>

<svelte:head>
	<title>Betreueranmeldung – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title">Betreueranmeldung für das Zeltlager</h1>

Warst du letztes Jahr schon angemeldet?<br />
Gib hier deine E-Mailadresse an. Du bekommst einen Link zugeschickt, mit dem du dich für dieses Jahr anmelden kannst.

<noscript>
Bitte aktiviere JavaScript
</noscript>

{#if error !== undefined}
	<div class="error-msg">
		<article class="message is-danger">
			<div class="message-body">
				{error}
			</div>
		</article>
	</div>
{/if}

<form
	class="form"
	action="#"
	on:submit|preventDefault={handleSubmit}
	bind:this={form}>
	<input
		id="mail"
		name="mail"
		placeholder="E-Mailadresse"
		required="required"
		class="input"
		autocomplete="email"
		inputmode="email"
		type="email" />
	<button
		type="submit"
		class="button is-primary"
		class:is-loading={isLoading && error === undefined}>
		Link schicken
	</button>
</form>

Noch nie angemeldet?
<a href="/intern/betreuer-anmeldung-neu">Hier kommst du zur Anmeldung</a>

<style>
	.form {
		display: flex;
		flex-wrap: wrap;
		gap: 1em;

		margin-top: 2em;
		margin-bottom: 2em;
	}

	.input {
		width: auto;
		flex-grow: 1;
		@media not screen and (max-width: 1230px) {
			max-width: 40em;
		}
	}
</style>
