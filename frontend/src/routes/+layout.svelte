<script lang="ts">
	import "@mdi/font/css/materialdesignicons.min.css";
	import { onMount } from "svelte";
	import { page } from "$app/stores";

	import "../app.scss";

	interface ExtraMenuItem {
		title: string;
		link: string;
	}

	interface MenuData {
		isLoggedIn: boolean;
		globalMessage: string | undefined;
		items: ExtraMenuItem[];
	}

	$: isWide = ["admin/betreuer", "admin/teilnehmer"].includes($page.routeId ?? "");

	let showNavbar = false;

	let menuData: MenuData = { isLoggedIn: false, globalMessage: undefined, items: [] };

	async function loadMenuItems() {
		const params = new URLSearchParams();
		if ($page.routeId !== null) {
			let routeId = $page.routeId;
			const i = routeId.indexOf("/");
			if (i !== -1) routeId = routeId.slice(0, i);
			params.append("prefix", routeId);
		}
		menuData = await (await fetch("/api/menu?" + params.toString())).json();
	}

	function stripSlashes(s: string) {
		if (s.startsWith("/")) s = s.slice(1);
		if (s.endsWith("/")) s = s.slice(0, -1);
		return s;
	}

	onMount(loadMenuItems);
</script>

<nav class="navbar is-light" aria-label="navigation">
	<div class="container" class:wide={isWide}>
		<div class="navbar-brand">
			<a class="navbar-brand" href="/">
				<img src="/img/MeinZeltlager.svg" style="padding: 0; height: 60px;" alt="Logo" />
			</a>
			<!-- svelte-ignore a11y-missing-attribute -->
			<a
				role="button"
				class="navbar-burger"
				aria-label="menu"
				aria-expanded={showNavbar}
				class:is-active={showNavbar}
				on:click={() => (showNavbar = !showNavbar)}>
				<span aria-hidden="true" />
				<span aria-hidden="true" />
				<span aria-hidden="true" />
			</a>
		</div>

		<div class="navbar-menu" class:is-active={showNavbar} on:click={() => (showNavbar = false)}>
			<div class="navbar-start">
				{#each menuData.items as item}
					{#if !["/anmeldung", "/packliste", "/ausstattung", "/betreuer", "/datenschutz", "/impressum"].includes(item.link)}
						<a
							class="navbar-item"
							href={item.link}
							class:is-active={$page.routeId === stripSlashes(item.link)}>
							<!-- set active for images links -->
							{item.title}
						</a>
					{/if}
				{/each}
				<a
					class="navbar-item"
					href="/anmeldung"
					class:is-active={$page.routeId === "anmeldung"}>Anmeldung</a>
				<a
					class="navbar-item"
					href="/packliste"
					class:is-active={$page.routeId === "packliste"}>Packliste</a>
				<a
					class="navbar-item"
					href="/ausstattung"
					class:is-active={$page.routeId === "ausstattung"}>Ausstattung und Team</a>
				<a
					class="navbar-item"
					href="/betreuer"
					class:is-active={$page.routeId === "betreuer"}>Für Betreuer</a>
				<a
					class="navbar-item"
					href="/datenschutz"
					class:is-active={$page.routeId === "datenschutz"}>Datenschutz</a>
				<a
					class="navbar-item"
					href="/impressum"
					class:is-active={$page.routeId === "impressum"}>Impressum</a>
			</div>

			<div class="navbar-end">
				<div class="navbar-item">
					<div class="buttons">
						<a
							class="button is-primary"
							href="/login"
							class:is-hidden={menuData.isLoggedIn}>
							<strong>Login</strong>
						</a>
						{#if menuData.isLoggedIn}
							<a class="button is-primary" href="/api/logout">
								<strong>Logout</strong>
							</a>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
</nav>

<div class="container main" class:wide={isWide}>
	{#if menuData.globalMessage !== undefined}
		<div class="globalMessage">
			{@html menuData.globalMessage}
		</div>
	{/if}

	<h4 class="subtitle is-4" style="text-align: center;">
		<a target="_blank" href="https://ftgern.de" style="color: black;">
			<img
				src="/img/GernerWappen.png"
				style="height: 2em; vertical-align: middle;"
				alt="FT Gern Wappen" />
			Freie Turnerschaft München Gern e.V.
		</a> – Zeltlager
	</h4>

	<slot />
</div>

<footer class="footer">
	<div class="has-text-centered">
		<a href="https://github.com/lacinoire/zeltlager-website" class="has-text-grey">
			Programmiert von Caro und Sebastian
		</a>
	</div>
</footer>

<style>
	@media screen and (max-width: 1023px) {
		.container.main {
			margin-left: 1em;
			margin-right: 1em;
		}
	}

	.container.main.wide {
		max-width: 100%;
		margin-left: 1em;
		margin-right: 1em;
	}

	.globalMessage :global(.message) {
		margin: 2em;
	}
</style>
