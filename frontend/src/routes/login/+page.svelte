<script lang="ts">
	let error: string | undefined;
	let showPassword: boolean = false;
	let passwordInput: HTMLInputElement | undefined;
	let loginForm: HTMLFormElement | undefined;

	function setShowPassword(showPassword: boolean) {
		if (passwordInput === undefined) return;
		passwordInput.setAttribute("type", showPassword ? "text" : "password");
	}

	$: setShowPassword(showPassword);

	async function login() {
		error = undefined;
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
				const urlParams = new URLSearchParams(window.location.search);
				let redirect = urlParams.get("redirect");
				if (redirect === null) redirect = "/";
				window.location.href = redirect;
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

<div style="max-width: 20em; margin-left: auto; margin-right: auto;">
	<form
		method="post"
		action="/api/login-nojs"
		on:submit|preventDefault={login}
		bind:this={loginForm}
	>
		<div class="field">
			<div class="control">
				<input
					id="username"
					name="username"
					placeholder="Name"
					required
					autocomplete="username"
					class="input"
					type="text"
				/>
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
					type="password"
				/>
			</div>
		</div>
		<div class="field">
			<div class="control">
				<label class="checkbox">
					<input
						id="showPassword"
						type="checkbox"
						bind:checked={showPassword}
						style="margin: 0.7em; vertical-align: middle;"
					/>
					Passwort anzeigen
				</label>
			</div>
		</div>
		<div class="field">
			<div class="control">
				<button class="button is-link" style="width: 100%;" type="submit">Anmelden</button>
			</div>
		</div>
	</form>
</div>
