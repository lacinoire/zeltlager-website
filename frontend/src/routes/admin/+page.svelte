<script lang="ts">
	import { tick } from "svelte";
	import { goto } from "$app/navigation";
	import { mdiDeleteOutline } from "@mdi/js";
	import Icon from "$lib/Icon.svelte";
	import { sleep } from "$lib/utils";

	let error: string | undefined = $state();
	let success: string | undefined = $state();

	let deleteModalOpen = $state(false);
	let deleteModalInput: HTMLInputElement | undefined = $state();
	let deleteModalLoading = $state(false);
	let deleteModalTeilnehmer: HTMLElement | undefined = $state();
	let deleteModalBetreuer: HTMLElement | undefined = $state();
	let deleteModalErwischtGames: HTMLElement | undefined = $state();
	let deleteIsLoading = $state(false);

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

	function onCloseDeleteModal(e) {
		e.preventDefault();
		closeDeleteModal();
	}

	function closeDeleteModal() {
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
				console.error("Failed to load data", resp);
				error = "Lager löschen fehlgeschlagen (" + (await resp.text()) + ")";
			}
			return;
		}

		success = "Lager erfolgreich gelöscht";
	}

	function documentKeyDown(event) {
		if (event.key === "Escape") {
			closeDeleteModal();
		}
	}
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
