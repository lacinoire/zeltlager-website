<script lang="ts">
	import { onMount, tick } from "svelte";
	import { goto } from "$app/navigation";
	import { mdiDeleteOutline, mdiPlus } from "@mdi/js";
	import Icon from "$lib/Icon.svelte";
	import { sleep } from "$lib/utils";

	interface ImageLink {
		name: string;
		user: string;
		url: string;
	}

	let error: string | undefined = $state(undefined);
	let success: string | undefined = $state(undefined);
	let users: string[] | undefined = $state(undefined);
	let imageLinks: ImageLink[] | undefined = $state(undefined);

	let deleteModalOpen = $state(false);
	let deleteModalInput: HTMLInputElement | undefined = $state();
	let deleteModalLoading = $state(false);
	let deleteModalTeilnehmer: HTMLElement | undefined = $state();
	let deleteModalBetreuer: HTMLElement | undefined = $state();
	let deleteModalErwischtGames: HTMLElement | undefined = $state();
	let deleteIsLoading = $state(false);

	let createImageLinkModalOpen = $state(false);
	let createImageLinkModalNameInput: HTMLInputElement | undefined = $state();
	let createImageLinkIsLoading = $state(false);

	let passwordModalOpen = $state(false);
	let passwordModalLoading = $state(false);
	let passwordModalInput: HTMLInputElement | undefined = $state();
	let passwordModalButton: HTMLButtonElement | undefined = $state();

	let createUserModalOpen = $state(false);
	let createUserModalLoading = $state(false);
	let createUserModalInput: HTMLInputElement | undefined = $state();
	let createUserModalButton: HTMLButtonElement | undefined = $state();

	async function fetchDeleteInfo() {
		deleteModalLoading = true;
		const resp = await fetch("/api/admin/lager");
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", resp);
				error = "Daten konnten nicht heruntergeladen werden. Hat der Account Admin-Rechte?";
			}
			return;
		}
		const data = await resp.json();
		deleteModalTeilnehmer.textContent = data.teilnehmer_count;
		deleteModalBetreuer.textContent = data.old_betreuer_count;
		deleteModalErwischtGames.textContent = data.erwischt_game_count;

		deleteModalLoading = false;
	}

	async function openDeleteModal(e) {
		e.preventDefault();
		deleteModalOpen = true;
		deleteIsLoading = false;
		success = undefined;
		if (deleteModalInput) {
			await tick();
			deleteModalInput.focus();
		}
		await fetchDeleteInfo();
	}

	function closeDeleteModal(e) {
		if (e !== undefined)
			e.preventDefault();
		deleteModalOpen = false;
		if (deleteModalInput !== undefined)
			deleteModalInput.value = "";
	}

	async function deleteLager(e) {
		e.preventDefault();
		if (deleteModalInput.value !== "Zeltlager") {
			alert("Bitte im Textfeld ‚Zeltlager‘ eingeben, um die Daten zu löschen.");
			return;
		}

		deleteIsLoading = true;
		closeDeleteModal();

		// Remove lager
		const resp = await fetch("/api/admin/lager", {
			method: "DELETE",
		});
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to remove lager", resp);
				error = "Lager löschen fehlgeschlagen (" + (await resp.text()) + ")";
			}
			deleteIsLoading = false;
			return;
		}

		deleteIsLoading = false;
		success = "Lager erfolgreich gelöscht";
	}

	function documentKeyDown(event) {
		if (event.key === "Escape") {
			closeDeleteModal();
			mailModalOpen = false;
		}
	}

	async function loadImageLinks() {
		imageLinks = await (await fetch("/api/admin/imageLink/list")).json();
	}

	async function createImageLink(e) {
		e.preventDefault();

		// Create link
		const params = new URLSearchParams();
		params.append("name", createImageLinkModalNameInput.value);

		createImageLinkIsLoading = true;
		closeCreateImageLinkModal();

		const resp = await fetch("/api/admin/imageLink?" + params.toString(), {
			method: "POST",
		});
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to create link", resp);
				error = "Link erstellen fehlgeschlagen (" + (await resp.text()) + ")";
			}
			createImageLinkIsLoading = false;
			return;
		}

		createImageLinkIsLoading = false;
		// May restart the server, so wait shortly
		setTimeout(loadImageLinks, 1000);
	}

	async function openCreateImageLinkModal(e) {
		e.preventDefault();
		createImageLinkModalOpen = true;
		createImageLinkIsLoading = false;
		success = undefined;
		if (createImageLinkModalNameInput) {
			await tick();
			createImageLinkModalNameInput.focus();
		}
	}

	function closeCreateImageLinkModal(e) {
		if (e !== undefined)
			e.preventDefault();
		createImageLinkModalOpen = false;
		if (createImageLinkModalNameInput !== undefined)
			createImageLinkModalNameInput.value = "";
	}

	async function deleteImageLink(link: ImageLink) {
		if (!window.confirm(`Link "${link.name}" löschen? Das kann nicht rückgängig gemacht werden.`))
			return;
		const params = new URLSearchParams();
		params.append("name", link.name);
		const resp = await fetch("/api/admin/imageLink?" + params.toString(), { method: "DELETE" });
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to delete link", resp);
				error = "Link konnte nicht gelöscht werden. Hat der Account Admin-Rechte?";
			}
			return;
		}
		loadImageLinks();
	}

	async function loadUsers() {
	const resp = await fetch("/api/admin/user/list");
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to fetch users", resp);
				error = "Admin Seite laden fehlgeschlagen (" + (await resp.text()) + ")";
			}
			deleteIsLoading = false;
			return;
		}
		users = await resp.json();
	}

	async function resetPassword(user) {
		passwordModalButton.innerHTML = "Kopieren";
		passwordModalLoading = true;
		passwordModalOpen = true;

		const params = new URLSearchParams();
		params.append("user", user);
		const resp = await fetch("/api/admin/user/reset_password?" + params.toString(), { method: "POST" });
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", resp);
				error = "Passwort konnte nicht zurückgesetzt werden. Hat der Account Admin-Rechte?";
				passwordModalOpen = false;
			}
			return;
		}
		const data = await resp.text();
		passwordModalInput.value = data;

		passwordModalLoading = false;
	}

	async function openCreateUserModal() {
		createUserModalOpen = true;
	}

	async function createUser() {
		createUserModalLoading = true;

		const params = new URLSearchParams();
		params.append("user", createUserModalInput.value);
		const resp = await fetch("/api/admin/user/create?" + params.toString(), { method: "POST" });
		if (!resp.ok) {
			// Unauthorized
			if (resp.status == 401) {
				goto("/login?redirect=" + encodeURIComponent(window.location.pathname));
			} else {
				console.error("Failed to load data", resp);
				error = "Benutzer konnte nicht erstellt werden. Hat der Account Admin-Rechte?";
				createUserModalLoading = false;
			}
			return;
		}

		createUserModalLoading = false;
		createUserModalOpen = false;
		loadUsers();
	}

	async function copyPasswordLink() {
		try {
			await navigator.clipboard.writeText(passwordModalInput.value);
			passwordModalButton.innerHTML = "✔";
		} catch (err) {
			console.log("Failed to copy", err);
			passwordModalButton.innerHTML = "✘ Kopieren fehlgeschlagen";
		}
	}

	onMount(() => {
		loadImageLinks();
		loadUsers();
	});
</script>

<svelte:document onkeydown={documentKeyDown} />

<svelte:head>
	<title>Verwaltung – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<h1 class="title is-1">Verwaltung</h1>

{#if error !== undefined}
	<article class="message is-danger">
		<div class="message-body">
			{error}
		</div>
	</article>
{/if}

{#if success !== undefined}
	<article class="message is-success">
		<div class="message-body">
			{success}
		</div>
	</article>
{/if}

<p class="buttons">
	<button
		class="button is-danger"
		onclick={openDeleteModal}>
		<span class="icon">
			<Icon name={mdiDeleteOutline} />
		</span>
		<span>
			Lager löschen
		</span>
	</button>
</p>

<div>
<h2 class="title is-2">Tabellen</h2>

<div class="is-flex galleryContainer">
	<a href="/admin/" class="box" style="display: none;"> <!-- TODO -->
		<div class="document">
			<span class="icon emojiIcon">
				🤐
			</span>
			Unverträglichkeiten
		</div>
	</a>

	<a href="/admin/" class="box" style="display: none;"> <!-- TODO -->
		<div class="document">
			<span class="icon emojiIcon">
				💊
			</span>
			Medikamente
		</div>
	</a>

	<a href="/admin/zuschuesse" class="box">
		<div class="document">
			<span class="icon emojiIcon">
				💸
			</span>
			Zuschüsse
		</div>
	</a>

	<a href="/admin/lagerkasse" class="box">
		<div class="document">
			<span class="icon emojiIcon">
				🪙
			</span>
			Lagerkasse
		</div>
	</a>
</div>
</div>

<h2 class="title is-2">Bilder</h2>

<p class="buttons">
	<button
		class="button"
		class:is-loading={createImageLinkIsLoading}
		onclick={openCreateImageLinkModal}>
		<span class="icon">
			<Icon name={mdiPlus} />
		</span>
		<span>
			Neuen Link erstellen
		</span>
	</button>
</p>

<div class="content">
<ul>
	{#each imageLinks as link}
		<li>
			<a href={link.url}>
				{link.name}:
			</a>
			<a
				class="has-text-danger"
				role="button"
				tabindex="0"
				onclick={() => deleteImageLink(link)}
				onkeydown={(e) => {
					if (e.key === "Enter") deleteImageLink(link);
				}}
			>
				Link löschen
			</a>
		</li>
	{/each}
</ul>
</div>

{#if users !== undefined}
<h2 class="title is-2">Administratoren</h2>

<div class="content">
<ul>
	{#each users as u}
		<li>
			{u}:
			<!-- svelte-ignore a11y_missing_attribute -->
			<a
				role="button"
				tabindex="0"
				onclick={() => resetPassword(u)}
				onkeydown={(e) => {
					if (e.key === "Enter") resetPassword(u);
				}}
			>
				Passwort zurücksetzen
			</a>
		</li>
	{/each}
	<li>
		<!-- svelte-ignore a11y_missing_attribute -->
		<a
			role="button"
			tabindex="0"
			onclick={openCreateUserModal}
			onkeydown={(e) => {
				if (e.key === "Enter") openCreateUserModal();
			}}
		>
			Neuen Administrator hinzufügen
		</a>
	</li>
	<li>
		<a href="/api/oauth2/provider" rel="external">
			Eigenes Passwort ändern
		</a>
	</li>
</ul>
</div>
{/if}

<div class="modal" class:is-active={deleteModalOpen}>
  <div class="modal-background" onclick={closeDeleteModal}></div>
  <form class="modal-card" onsubmit={deleteLager}>
    <header class="modal-card-head">
      <p class="modal-card-title">Lager löschen</p>
      <button type="button" class="delete" aria-label="close" onclick={closeDeleteModal}></button>
    </header>
    <section class="modal-card-body">
    	<div class="content">
	      <strong>Achtung:</strong> Hiermit werden gelöscht:
	      <ul>
	      	<li>Alle <strong class:is-skeleton={deleteModalLoading} bind:this={deleteModalTeilnehmer}>100</strong> Teilnehmer</li>
	      	<li><strong class:is-skeleton={deleteModalLoading} bind:this={deleteModalBetreuer}>100</strong> Betreuer, die dieses Jahr nicht angemeldet waren</li>
	      	<li>Alle <strong class:is-skeleton={deleteModalLoading} bind:this={deleteModalErwischtGames}>100</strong> Erwischt Spiele</li>
	      </ul>
	    </div>
	    Hier <code>Zeltlager</code> eingeben zum löschen:
    	<input type="text" bind:this={deleteModalInput} />
    </section>
    <footer class="modal-card-foot">
      <div class="buttons">
        <button class="button is-danger" type="submit" class:is-loading={deleteIsLoading}>Daten löschen</button>
        <button class="button" onclick={closeDeleteModal}>Abbrechen</button>
      </div>
    </footer>
  </form>
</div>

<div class="modal" class:is-active={createImageLinkModalOpen}>
  <div class="modal-background" onclick={closeCreateImageLinkModal}></div>
  <form class="modal-card" onsubmit={createImageLink}>
    <header class="modal-card-head">
      <p class="modal-card-title">Link für Bilder erstellen</p>
      <button type="button" class="delete" aria-label="close" onclick={closeCreateImageLinkModal}></button>
    </header>
    <section class="modal-card-body">
    	<div class="content">
    		<div>
		    	Name (z.B. „Bilder2026“ oder „BilderBetreuer2026“):
		    	<input type="text" bind:this={createImageLinkModalNameInput} />
    		</div>
	    </div>
    </section>
    <footer class="modal-card-foot">
      <div class="buttons">
        <button class="button is-primary" type="submit" class:is-loading={createImageLinkIsLoading}>Link erstellen</button>
        <button class="button" onclick={closeCreateImageLinkModal}>Abbrechen</button>
      </div>
    </footer>
  </form>
</div>

<div class="modal" class:is-active={passwordModalOpen}>
  <div class="modal-background" onclick={() => passwordModalOpen = false}></div>
  <div class="modal-content">
  	<div class="box">
  		<div class="field has-addons">
			  <div class="control" style="flex-grow: 1;">
			    <input class="input" class:is-skeleton={passwordModalLoading} type="text" bind:this={passwordModalInput} />
			  </div>
			  <div class="control">
			    <button class="button is-info" class:is-skeleton={passwordModalLoading} onclick={copyPasswordLink} bind:this={passwordModalButton}>
			    	Kopieren
			    </button>
			  </div>
			</div>
	  </div>
  </div>
  <button class="modal-close is-large" aria-label="close" onclick={() => passwordModalOpen = false}></button>
</div>

<div class="modal" class:is-active={createUserModalOpen}>
  <div class="modal-background" onclick={() => createUserModalOpen = false}></div>
  <div class="modal-content">
  	<div class="box">
  		<div class="field has-addons">
			  <div class="control" style="flex-grow: 1;">
			    <input class="input" class:is-skeleton={createUserModalLoading} type="text" bind:this={createUserModalInput} />
			  </div>
			  <div class="control">
			    <button class="button is-info" class:is-skeleton={createUserModalLoading} onclick={createUser} bind:this={createUserModalButton}>
			    	Administrator hinzufügen
			    </button>
			  </div>
			</div>
	  </div>
  </div>
  <button class="modal-close is-large" aria-label="close" onclick={() => createUserModalOpen = false}></button>
</div>

<style lang="scss">
	.galleryContainer {
		gap: 0.75em;
		flex-wrap: wrap;

		.box {
			padding: 0.5em;
			min-width: 150px;
			min-height: 150px;
			&:last-child {
				margin-bottom: 1.5rem;
			}
		}
	}

	.document {
		display: flex;
		flex-direction: column;
		text-align: center;
		padding: 1em;
	}

	.emojiIcon {
		font-size: 6em;
		margin: 0.1em;
	}
</style>
