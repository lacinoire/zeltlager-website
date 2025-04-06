<script lang="ts">
	import { onMount } from "svelte";

	let mail: string | undefined;

	onMount(() => {
  	const urlParams = new URLSearchParams(window.location.search);
  	let text = urlParams.get("mail");
  	if (text)
  	  mail = text;
	});
</script>

<svelte:head>

<title>Betreueranmeldung – Zeltlager – FT München Gern e.V.</title>
</svelte:head>

<div class="content">

<h1>E-Mail wurde verschickt</h1>

Wenn
{#if mail === undefined}
die E-Mail-Adresse
{:else}
<b>{mail}</b>
{/if}
als Betreuer eingetragen ist, bekommst du eine E-Mail mit einem Link, um dich für dieses Jahr anzumelden.
Solltest du keine E-Mail bekommen, schau in deinen Spam-Ordner oder melde dich neu mit all deinen Daten an.

</div>
