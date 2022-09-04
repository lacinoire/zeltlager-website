// Show error if there is one
<div class="alert alert-danger" role="alert" />

<h1 class="headline">Betreueranmeldung für das Zeltlager</h1>

<form style="padding-top: 10px" method="post" action="/intern/signup-supervisor-send">
	<div class="form-group form-row required">
		<label for="vorname" class="col-4 col-form-label">Vorname</label>
		<div class="col-8">
			<input
				id="vorname"
				name="vorname"
				placeholder="Vorname"
				required
				class="form-control here"
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<label for="nachname" class="col-4 col-form-label">Nachname</label>
		<div class="col-8">
			<input
				id="nachname"
				name="nachname"
				placeholder="Nachname"
				class="form-control here"
				required
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<label for="geburtsdatum" class="col-4 col-form-label">Geburtsdatum</label>
		<div class="col-8">
			<input
				id="geburtsdatum"
				name="geburtsdatum"
				placeholder="TT.MM.JJJJ"
				class="form-control here"
				required
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<label class="col-4 col-form-label">Geschlecht</label>
		<div class="col-8">
			<label class="custom-control custom-radio">
				<input
					name="geschlecht"
					class="custom-control-input"
					value="m"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Männlich</span>
			</label>
			<label class="custom-control custom-radio">
				<input
					name="geschlecht"
					class="custom-control-input"
					value="w"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Weiblich</span>
			</label>
		</div>
	</div>
	<div class="form-group form-row required">
		<label class="col-4 col-form-label">Vegetarier</label>
		<div class="col-8">
			<label class="custom-control custom-radio">
				<input
					name="vegetarier"
					class="custom-control-input"
					value="true"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Ja</span>
			</label>
			<label class="custom-control custom-radio">
				<input
					name="vegetarier"
					class="custom-control-input"
					value="false"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Nein</span>
			</label>
		</div>
	</div>
	<div class="form-group form-row required">
		<label class="col-4 col-form-label">Tetanusimpfung</label>
		<div class="col-8">
			<label class="custom-control custom-radio">
				<input
					name="tetanus_impfung"
					class="custom-control-input"
					value="true"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Ja</span>
			</label>
			<label class="custom-control custom-radio">
				<input
					name="tetanus_impfung"
					class="custom-control-input"
					value="false"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Nein</span>
			</label>
		</div>
	</div>
	<div class="form-group form-row required">
		<label class="col-4 col-form-label">Krankenversicherung</label>
		<div class="col-8">
			<label class="custom-control custom-radio">
				<input
					name="krankenversicherung"
					class="custom-control-input"
					value="gesetzlich"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Gesetzlich</span>
			</label>
			<label class="custom-control custom-radio">
				<input
					name="krankenversicherung"
					class="custom-control-input"
					value="privat"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Privat</span>
			</label>
			<label class="custom-control custom-radio">
				<input
					name="krankenversicherung"
					class="custom-control-input"
					value="anderes"
					required
					type="radio"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description">Anderes</span>
			</label>
		</div>
	</div>
	<div class="form-group form-row">
		<label for="juleica_nummer" class="col-4 col-form-label">Juleica Nummer</label>
		<div class="col-8">
			<input
				id="juleica_nummer"
				name="juleica_nummer"
				placeholder="Juleicanummer"
				class="form-control here"
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<label for="mail" class="col-4 col-form-label">E-Mailadresse</label>
		<div class="col-8">
			<input
				id="mail"
				name="mail"
				placeholder="E-Mailadresse"
				class="form-control here"
				required
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<label for="handynummer" class="col-4 col-form-label">Handynummer</label>
		<div class="col-8">
			<input
				id="handynummer"
				name="handynummer"
				placeholder="Handynummer"
				class="form-control here"
				required
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<div class="col-4">
			<label for="strasse" class="col-form-label">Straße</label>,
			<label for="hausnummer" class="col-form-label">Hausnummer</label>
		</div>
		<div class="col-6">
			<input
				id="strasse"
				name="strasse"
				placeholder="Straße"
				class="form-control here"
				required
				type="text"
			/>
		</div>
		<div class="col-2">
			<input
				id="hausnummer"
				name="hausnummer"
				placeholder="Hausnummer"
				class="form-control here"
				required
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<div class="col-4">
			<label for="plz" class="col-form-label">Postleitzahl</label>,
			<label for="ort" class="col-form-label">Ort</label>
		</div>
		<div class="col-2">
			<input
				id="plz"
				name="plz"
				placeholder="PLZ"
				class="form-control here"
				required
				type="text"
			/>
		</div>
		<div class="col-6">
			<input
				id="ort"
				name="ort"
				placeholder="Ort"
				class="form-control here"
				required
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row">
		<label for="fuehrungszeugnis_auststellung" class="col-4 col-form-label"
			>Austellungsdatum Erweitertes Führungszeugnis</label
		>
		<div class="col-8">
			<input
				id="fuehrungszeugnis_auststellung"
				name="fuehrungszeugnis_auststellung"
				placeholder="TT.MM.JJJJ"
				class="form-control here"
				type="text"
			/>
		</div>
	</div>
	<div class="form-group form-row required">
		<label class="col-4 col-form-label"
			>Selbstverpflichtungserklärung zur Prävention von sexualisierter Gewalt</label
		>
		<div class="col-8">
			<label class="custom-control custom-checkbox">
				<input
					name="selbsterklaerung"
					class="custom-control-input"
					value="true"
					required
					type="checkbox"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description"
					>Ich habe die <a href="/intern/selbstverpflichtung" target="_blank"
						>Selbstverpflichtungserklärung zur Prävention von sexualisierter Gewalt</a
					>
					sowie den
					<a href="/intern/selbstverpflichtung-anhang" target="_blank">Anhang</a> gelesen und
					verpflichte mich, mich daran zu halten.</span
				>
			</label>
		</div>
	</div>
	<div class="form-group form-row">
		<label for="allergien" class="col-4 col-form-label">Allergien</label>
		<div class="col-8">
			<textarea
				id="allergien"
				name="allergien"
				cols="40"
				rows="1"
				class="form-control"
				aria-describedby="allergienHelpBlock"
			/>
			<span id="allergienHelpBlock" class="form-text text-muted">z.B. Haselnussallergie</span>
		</div>
	</div>
	<div class="form-group form-row">
		<label for="unvertraeglichkeiten" class="col-4 col-form-label"
			>Lebensmittelunverträglichkeiten</label
		>
		<div class="col-8">
			<textarea
				id="unvertraeglichkeiten"
				name="unvertraeglichkeiten"
				cols="40"
				rows="1"
				class="form-control"
				aria-describedby="unvertraeglichkeitenHelpBlock"
			/>
			<span id="unvertraeglichkeitenHelpBlock" class="form-text text-muted"
				>z.B. Laktoseintoleranz, kein Schweinefleisch</span
			>
		</div>
	</div>
	<div class="form-group form-row">
		<label for="medikamente" class="col-4 col-form-label">Medikamente</label>
		<div class="col-8">
			<textarea
				id="medikamente"
				name="medikamente"
				cols="40"
				rows="1"
				class="form-control"
				aria-describedby="medikamenteHelpBlock"
			/>
			<span id="medikamenteHelpBlock" class="form-text text-muted"
				>z.B. Asthmaspray; Methylphenidat, 10 mg</span
			>
		</div>
	</div>
	<div class="form-group form-row">
		<label for="besonderheiten" class="col-4 col-form-label">Besonderheiten</label>
		<div class="col-8">
			<textarea
				id="besonderheiten"
				name="besonderheiten"
				cols="40"
				rows="2"
				class="form-control"
				aria-describedby="besonderheitenHelpBlock"
			/>
			<span id="besonderheitenHelpBlock" class="form-text text-muted"
				>Krankheiten, Eigenheiten, etc.</span
			>
		</div>
	</div>
	<div class="form-group form-row required">
		<label class="col-4 col-form-label"
			>Allgemeine Geschäftsbedingungen und Datenschutzbestimmungen</label
		>
		<div class="col-8">
			<label class="custom-control custom-checkbox">
				<input
					name="agb"
					class="custom-control-input"
					value="true"
					required
					type="checkbox"
				/>
				<span class="custom-control-indicator" />
				<span class="custom-control-description"
					>Ich habe die <a href="/agb" target="_blank">AGB</a> und die
					<a href="/datenschutz" target="_blank">Datenschutzbestimmungen</a> gelesen und akzeptiere
					sie.</span
				>
			</label>
		</div>
	</div>
	<div class="form-group form-row">
		<div class="offset-4 col-8">
			<button name="submit" type="submit" class="btn btn-primary">Anmelden</button>
		</div>
	</div>
</form>
