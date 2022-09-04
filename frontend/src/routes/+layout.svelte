<script lang="ts">
	import "@mdi/font/css/materialdesignicons.min.css";

	import "../app.scss";

	let showNavbar = false;
</script>

<nav class="navbar" aria-label="navigation">
	<div class="container">
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
				on:click={() => (showNavbar = !showNavbar)}
				data-target="mainNavbar">
				<span aria-hidden="true" />
				<span aria-hidden="true" />
				<span aria-hidden="true" />
			</a>
		</div>

		<div id="mainNavbar" class="navbar-menu" class:is-active={showNavbar}>
			<!-- TODO Mark active header -->
			<!-- TODO Add custom headers -->
			<div class="navbar-start">
				<a class="navbar-item" href="/anmeldung">Anmeldung</a>
				<a class="navbar-item" href="/packliste">Packliste</a>
				<a class="navbar-item" href="/ausstattung">Ausstattung</a>
				<a class="navbar-item" href="/betreuer">Für Betreuer</a>
				<a class="navbar-item" href="/datenschutz">Datenschutz</a>
				<a class="navbar-item" href="/impressum">Impressum</a>
			</div>

			<div class="navbar-end">
				<div class="navbar-item">
					<div class="buttons">
						<a class="button is-primary" href="/login">
							<strong>Login</strong>
						</a>
					</div>
				</div>
			</div>
		</div>
	</div>
</nav>

<div class="container">
	TODO Show global message if there is one

	<h4 class="subtitle is-4" style="text-align: center;">
		<a target="_blank" href="https://ftgern.de" style="color: black;">
			<!-- TODO align wappen -->
			<img src="/img/GernerWappen.png" style="height: 2em;" alt="FT Gern Wappen" />
			Freie Turnerschaft München Gern e.V.
		</a> – Zeltlager
	</h4>

	<slot />
</div>

<!-- TODO Always move footer to the very bottom -->
<footer class="footer">
	<div class="has-text-centered">
		<a href="https://github.com/lacinoire/zeltlager-website" class="has-text-grey">
			Programmiert von Caro und Sebastian
		</a>
	</div>
</footer>
