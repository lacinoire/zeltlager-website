<script lang="ts">
	import { tick } from "svelte";
	import { goto } from "$app/navigation";
	import { mdiDeleteOutline } from "@mdi/js";
	import Icon from "$lib/Icon.svelte";
	import { sleep } from "$lib/utils";

	let error: string | undefined;
	let mailModalOpen = false;
	let mailModalLoading = false;
	let mailModalInput: HTMLInputElement | undefined;
	let mailModalButton: HTMLButtonElement | undefined;

	let deleteModalOpen = false;
	let deleteModalInput: HTMLInputElement | undefined;

	async function openMailModal() {
		mailModalButton.innerHTML = "Kopieren";
		mailModalLoading = true;
		mailModalOpen = true;

		// TODO Fetch mails, separate by ;

		mailModalLoading = false;
	}

	async function copyMails() {
		try {
			await navigator.clipboard.writeText(mailModalInput.value);
			mailModalButton.innerHTML = "✔";
		} catch (err) {
			console.log("Failed to copy", err);
			mailModalButton.innerHTML = "✘ Kopieren fehlgeschlagen";
		}
	}

	async function openDeleteModal() {
		deleteModalOpen = true;
		if (deleteModalInput) {
			await tick();
			deleteModalInput.focus();
		}
	}

	function closeDeleteModal(ev: Event) {
		deleteModalOpen = false;
		if (deleteModalInput !== undefined)
			deleteModalInput.value = "";
	}

	function deleteLager() {
		if (deleteModalInput.value !== "Zeltlager") {
			alert("Bitte im Textfeld ‚Zeltlager‘ eingeben, um die Daten zu löschen.");
			return;
		}

		// TODO Remove lager and show success
		closeDeleteModal();
	}

  function documentKeyDown(event) {
    if (event.key === "Escape") {
      mailModalOpen = false;
      closeDeleteModal();
    }
  }
</script>

<svelte:document on:keydown={documentKeyDown} />

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

<p class="buttons">
	<button
		class="button is-info"
		on:click|preventDefault={openMailModal}>
    <span class="icon">
			✉️
		</span>
		<span>
			Eltern E-Mailadressen anzeigen
		</span>
	</button>
</p>

<p class="buttons">
	<button
		class="button is-danger"
		on:click|preventDefault={openDeleteModal}>
    <span class="icon">
			<Icon name={mdiDeleteOutline} />
		</span>
		<span>
			Lager löschen
		</span>
	</button>
</p>

<h2 class="title is-2">Tabellen</h2>

<div class="is-flex galleryContainer">
	<a href="./" class="box"> <!-- TODO -->
		<div class="document">
	    <span class="icon emojiIcon">
				🤐
			</span>
			Unverträglichkeiten
		</div>
	</a>

	<a href="./" class="box"> <!-- TODO -->
		<div class="document">
	    <span class="icon emojiIcon">
				💊
			</span>
			Medikamente
		</div>
	</a>

	<a href="./" class="box"> <!-- TODO -->
		<div class="document">
	    <span class="icon emojiIcon">
				💸
			</span>
			Zuschüsse
		</div>
	</a>
</div>

<div class="modal" class:is-active={mailModalOpen}>
  <div class="modal-background" on:click={() => mailModalOpen = false}></div>
  <div class="modal-content">
  	<div class="box">
  		<div class="field has-addons">
			  <div class="control" style="flex-grow: 1;">
			    <input class="input" class:is-skeleton={mailModalLoading} type="text" bind:this={mailModalInput} />
			  </div>
			  <div class="control">
			    <button class="button is-info" class:is-skeleton={mailModalLoading} on:click={copyMails} bind:this={mailModalButton}>
			    	Kopieren
			    </button>
			  </div>
			</div>
	  </div>
  </div>
  <button class="modal-close is-large" aria-label="close" on:click={() => mailModalOpen = false}></button>
</div>

<div class="modal" class:is-active={deleteModalOpen}>
  <div class="modal-background" on:click={closeDeleteModal}></div>
  <form class="modal-card" on:submit|preventDefault={deleteLager}>
    <header class="modal-card-head">
      <p class="modal-card-title">Lager löschen</p>
      <button type="button" class="delete" aria-label="close" on:click={closeDeleteModal}></button>
    </header>
    <section class="modal-card-body">
    	<div class="content">
	      <strong>Achtung:</strong> Hiermit werden gelöscht:
	      <ul>
	      	<li>Alle TODO Teilnehmer</li>
	      	<li>TODO Betreuer, die dieses Jahr nicht angemeldet waren</li>
	      	<li>Alle TODO Erwischt Spiele</li>
	      </ul>
	    </div>
	    Hier <code>Zeltlager</code> eingeben zum löschen:
    	<input type="text" bind:this={deleteModalInput} />
    </section>
    <footer class="modal-card-foot">
      <div class="buttons">
        <button class="button is-danger" type="submit">Daten löschen</button>
        <button class="button" on:click={() => console.log("Closing??")}>Abbrechen</button>
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
