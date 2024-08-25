<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { mdiDeleteOutline } from "@mdi/js";
	import Icon from "$lib/Icon.svelte";

	let error: string | undefined;
	let deleteModal: HTMLDivElement | undefined;
	let deleteModalOpen = false;

	function showMails() {
		
	}

  function documentKeyDown(event) {
    if (event.key === "Escape") {
      deleteModalOpen = false;
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
		on:click|preventDefault={showMails}>
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
		on:click|preventDefault={() => deleteModalOpen = true}>
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

<div class="modal" bind:this={deleteModal} class:is-active={deleteModalOpen}>
  <div class="modal-background" on:click={() => deleteModalOpen = false}></div>
  <div class="modal-card">
    <header class="modal-card-head">
      <p class="modal-card-title">Lager löschen</p>
      <button class="delete" aria-label="close" on:click={() => deleteModalOpen = false}></button>
    </header>
    <section class="modal-card-body content">
      <strong>Achtung:</strong> Hiermit werden gelöscht:
      <ul>
      	<li>Alle TODO Teilnehmer</li>
      	<li>TODO Betreuer, die dieses Jahr nicht angemeldet waren</li>
      	<li>Alle TODO Erwischt Spiele</li>
      </ul>
    </section>
    <footer class="modal-card-foot">
      <div class="buttons">
        <button class="button is-danger">Daten löschen</button>
        <button class="button" on:click={() => deleteModalOpen = false}>Abbrechen</button>
      </div>
    </footer>
  </div>
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
