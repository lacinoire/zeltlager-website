<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import { browser } from "$app/environment";
	import { afterNavigate } from "$app/navigation";

	import "../app.scss";

	interface ExtraMenuItem {
		title: string;
		link: string;
	}

	interface MenuData {
		isLoggedIn: boolean;
		globalMessage: string | null;
		items: ExtraMenuItem[];
	}

	$: isWide = ["/admin/betreuer", "/admin/teilnehmer"].includes($page.route.id ?? "");

	let showNavbar = false;
	let menuData: MenuData = { isLoggedIn: false, globalMessage: null, items: [] };
	let location: string = browser ? window.location.pathname : "";

	async function loadMenuItems() {
		const params = new URLSearchParams();
		if ($page.route.id !== null) {
			let routeId = $page.route.id;
			const i = routeId.lastIndexOf("/");
			if (i !== -1) routeId = routeId.slice(0, i);
			params.append("prefix", routeId);
		}
		menuData = await (await fetch("/api/menu?" + params.toString())).json();
	}

	function stripSlashes(s: string | null | undefined) {
		if (!s) return s;
		if (s.startsWith("/")) s = s.slice(1);
		if (s.endsWith("/")) s = s.slice(0, -1);
		return s;
	}

	onMount(loadMenuItems);

	afterNavigate(() => (location = browser ? window.location.pathname : ""));
</script>

<nav class="navbar is-fixed-top is-light" aria-label="main navigation">
	<div class="container" class:wide={isWide}>
		<div class="navbar-brand">
			<a class="navbar-brand" href="/">
				<img src="/img/MeinZeltlager.svg" style="padding: 0; height: 60px;" alt="Logo" />
			</a>
			<!-- svelte-ignore a11y-missing-attribute -->
	    <a role="button" class="navbar-burger" aria-label="menu"
				tabindex="0"
				aria-expanded={showNavbar}
				class:is-active={showNavbar}
				on:click={() => (showNavbar = !showNavbar)}
				on:keydown={(e) => {
					if (e.key === "Enter") showNavbar = !showNavbar;
				}}>
	      <span aria-hidden="true"></span>
	      <span aria-hidden="true"></span>
	      <span aria-hidden="true"></span>
	      <span aria-hidden="true"></span>
	    </a>
		</div>

		<div
			class="navbar-menu"
			class:is-active={showNavbar}
			on:click={() => (showNavbar = false)}
			on:keydown={(e) => {
				if (e.key === "Enter") showNavbar = false;
			}}>
			<div class="navbar-start">
				{#each menuData.items as item}
					{#if !["/anmeldung", "/packliste", "/ausstattung", "/betreuer"].includes(item.link)}
						<a
							class="navbar-item is-tab"
							href={item.link}
							class:is-active={stripSlashes($page.route.id) ===
								stripSlashes(item.link) ||
								stripSlashes(location) === stripSlashes(item.link)}>
							<!-- set active for images links -->
							{item.title}
						</a>
					{/if}
				{/each}
				{#if $page.route.id?.startsWith("/intern")}
					<a
						class="navbar-item is-tab"
						href="/intern"
						class:is-active={$page.route.id === "/intern"}>
						Betreuer-Info
					</a>
					<a
						class="navbar-item is-tab"
						href="/intern/betreuer-anmeldung"
						class:is-active={$page.route.id === "/intern/betreuer-anmeldung"}>
						Betreuer-Anmeldung
					</a>
					<a
						class="navbar-item is-tab"
						href="/intern/zuschuesse"
						class:is-active={$page.route.id === "/intern/zuschuesse"}>
						Zuschüsse
					</a>
				{/if}
				<a
					class="navbar-item is-tab emph-item"
					href="/anmeldung"
					class:is-active={$page.route.id === "/anmeldung"}>
					Anmeldung
				</a>
				<a
					class="navbar-item is-tab"
					href="/packliste"
					class:is-active={$page.route.id === "/packliste"}>
					Packliste
				</a>
				<a
					class="navbar-item is-tab"
					href="/ausstattung"
					class:is-active={$page.route.id === "/ausstattung"}>
					Ausstattung und Team
				</a>
				<a
					class="navbar-item is-tab"
					href="/betreuer"
					class:is-active={$page.route.id === "/betreuer"}>
					Für Betreuer
				</a>
				<a
					class="navbar-item is-tab emph-item"
					href="/login"
					class:is-hidden={menuData.isLoggedIn}>
					Login
				</a>
				{#if menuData.isLoggedIn}
					<a class="navbar-item is-tab emph-item" href="/api/logout">Logout</a>
				{/if}
			</div>
		</div>
	</div>
</nav>

<div class="container main" class:wide={isWide}>
	{#if menuData.globalMessage !== null}
		<div class="globalMessage">
			{@html menuData.globalMessage}
		</div>
	{/if}

	<slot />
</div>

<footer class="footer">
	<div class="content has-text-centered">
		<div style="margin-bottom: 2em; font-size: 1.2rem;">
			<a href="/datenschutz">Datenschutz</a>
			•
			<a href="/impressum">Impressum</a>
		</div>

		<div style="margin-bottom: 2em; font-size: 1.2rem;">
			<a href="https://ftgern.de">
				<img
					src="/img/GernerWappen.png"
					style="height: 2em; vertical-align: middle;"
					alt="FT Gern Wappen" />
				Freie Turnerschaft München Gern e.V.
			</a>
		</div>

		<div>
			<a href="https://github.com/lacinoire/zeltlager-website" class="has-text-grey">
				Programmiert von Caro und Sebastian
			</a>
		</div>
	</div>
</footer>

<style>
	@media screen and (max-width: 1023px) {
		.container.main {
			padding-left: 1em;
			padding-right: 1em;
		}
	}

	div.navbar-brand {
		align-items: center;
	}

	.container.main {
		padding-top: 3em;
	}

	.container.main.wide {
		max-width: 100%;
		padding-left: 1em;
		padding-right: 1em;
	}

	.emph-item {
		font-weight: bold;
	}

	.globalMessage :global(.message) {
		margin: 2em;
	}

	@media print {
		nav,
		.navbar-brand,
		footer {
			display: none;
		}

		.container.main {
			padding-top: 0;
		}

		:global(html body.has-navbar-fixed-top) {
			padding-top: 0;
		}
	}
</style>
