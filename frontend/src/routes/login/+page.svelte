// Show error if there is one
<div class="alert alert-danger" role="alert" />

<h1 class="headline" style="text-align: center;">Anmelden</h1>

<form class="login-form" style="padding-top: 10px;" method="post" action="login">
	<input
		id="username"
		name="username"
		placeholder="Name"
		required
		class="form-control here"
		type="text"
	/>
	<input
		id="password"
		name="password"
		placeholder="Passwort"
		required
		class="form-control here"
		type="password"
	/>
	<label
		><input
			id="showPassword"
			class="here"
			type="checkbox"
			style="margin: 0.7em; vertical-align: middle;"
		/>
		Passwort anzeigen
	</label>
	<!-- if let Some(redirect) = self.values.get("redirect") { -->
	<input name="redirect" type="hidden" value="<#= crate::escape_html_attribute(redirect) #>" />
	<button class="form-control here btn-primary" type="submit">Anmelden</button>
</form>
