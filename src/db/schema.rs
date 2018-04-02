table! {
	teilnehmer (id) {
		id -> Int4,
		vorname -> Text,
		nachname -> Text,
		geburtsdatum -> Date,
		geschlecht -> Text,
		schwimmer -> Bool,
		vegetarier -> Bool,
		tetanus_impfung -> Bool,
		eltern_name -> Text,
		eltern_mail -> Text,
		eltern_handynummer -> Text,
		strasse -> Text,
		hausnummer -> Text,
		ort -> Text,
		plz -> Text,
		besonderheiten -> Text,
		agb -> Bool,
	}
}
