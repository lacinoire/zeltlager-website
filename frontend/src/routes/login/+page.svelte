<script lang="ts">
	import { browser } from '$app/environment';

	let error: string | undefined = $state();
	let isLoading = $state(false);
	let showPassword: boolean = $state(false);
	let passwordInput: HTMLInputElement | undefined = $state();
	let loginForm: HTMLFormElement | undefined = $state();

	$effect(() => {
		if (passwordInput === undefined) return;
		passwordInput.setAttribute("type", showPassword ? "text" : "password");
	});

	function getRedirect() {
		if (!browser) return "/";
		let redirect = new URLSearchParams(window.location.search).get("redirect");
		if (redirect === null) redirect = "/";
		return redirect;
	}

	async function login(e) {
		e.preventDefault();
		if (isLoading && error === undefined) return;
		error = undefined;
		isLoading = true;

		let response: Response;
		try {
			response = await fetch("/api/login", {
				method: "POST",
				headers: {
					"Content-Type": "application/x-www-form-urlencoded; charset=utf-8",
				},
				body: new URLSearchParams(new FormData(loginForm) as any),
			});
		} catch (e) {
			console.error("Failed to make login web request", e);
			error = "Verbindung fehlgeschlagen. Ist das Internet erreichbar?";
			return;
		}
		let respText: string;
		try {
			respText = await response.text();
		} catch (e) {
			console.error("Failed to read login response", e);
			error = "Verbindung abgebrochen";
			return;
		}
		try {
			const resp = JSON.parse(respText);
			if (resp.error !== null) {
				error = resp.error;
			} else {
				// Login successful
				window.location.href = getRedirect();
			}
		} catch (e) {
			console.error("Failed to convert login request to json", e);
			error = respText;
			return;
		}
	}
</script>

<svelte:head>
	<title>Login – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title" style="text-align: center;">Anmelden</h1>

{#if error !== undefined}
	<article class="message is-danger">
		<div class="message-body">
			{error}
		</div>
	</article>
{/if}

<div class="centered">
	<article class="message is-info">
		<div class="message-body">
			Wenn Sie einen Link statt einem Benutzernamen haben, verwenden Sie den Link zum Anmelden.
		</div>
	</article>
</div>

<div style="max-width: 20em; margin-left: auto; margin-right: auto;">
	<form
		method="post"
		action="/api/login-nojs"
		onsubmit={login}
		bind:this={loginForm}>
		<div class="field">
			<div class="control">
				<input
					id="username"
					name="username"
					placeholder="Name"
					required
					autocomplete="username"
					class="input"
					type="text" />
			</div>
		</div>
		<div class="field">
			<div class="control">
				<input
					id="password"
					name="password"
					placeholder="Passwort"
					required
					autocomplete="current-password"
					class="input"
					bind:this={passwordInput}
					type="password" />
			</div>
		</div>
		<div class="field">
			<div class="control">
				<label class="checkbox">
					<input
						id="showPassword"
						type="checkbox"
						bind:checked={showPassword}
						style="margin: 0.7em; vertical-align: middle;" />
					Passwort anzeigen
				</label>
			</div>
		</div>
		<div class="field">
			<div class="control">
				<button
					class="button is-primary"
					style="width: 100%;"
					type="submit"
					class:is-loading={isLoading && error === undefined}>
					Anmelden
				</button>
			</div>
		</div>
	</form>
</div>

<div class="centered">
	<a
		href={"/api/oauth2/login?redirect=" + getRedirect()}
		rel="external">
		Admin Login
	</a>
</div>

<style>
	.centered {
		margin-top: 2em;
		margin-bottom: 2em;
		width: 100%;
		text-align: center;
	}
</style>
