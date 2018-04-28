table! {
	betreuer (id) {
		id -> Int4,
		vorname -> Text,
		nachname -> Text,
		geburtsdatum -> Date,
		geschlecht -> Text,
		vegetarier -> Bool,
		tetanus_impfung -> Bool,
		juleica_nummer -> Text,
		mail -> Text,
		handynummer -> Text,
		strasse -> Text,
		hausnummer -> Text,
		ort -> Text,
		plz -> Text,
		besonderheiten -> Text,
		agb -> Bool,
		selbsterklaerung -> Bool,
		fuehrungszeugnis_auststellung -> Date,
		fuehrungszeugnis_eingesehen -> Nullable<Date>,
	}
}

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

allow_tables_to_appear_in_same_query!(betreuer, teilnehmer,);
