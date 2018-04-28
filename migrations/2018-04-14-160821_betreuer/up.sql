-- Your SQL goes here
CREATE TABLE betreuer (
	id SERIAL PRIMARY KEY,
	vorname TEXT NOT NULL,
	nachname TEXT NOT NULL,
	geburtsdatum DATE NOT NULL,
	geschlecht TEXT NOT NULL,
	vegetarier BOOLEAN NOT NULL,
	tetanus_impfung BOOLEAN NOT NULL,
	juleica_nummer TEXT NOT NULL,
	mail TEXT NOT NULL,
	handynummer TEXT NOT NULL,
	strasse TEXT NOT NULL,
	hausnummer TEXT NOT NULL,
	ort TEXT NOT NULL,
	plz TEXT NOT NULL,
	besonderheiten TEXT NOT NULL,
	agb BOOLEAN NOT NULL,
	selbsterklaerung BOOLEAN NOT NULL,
	fuehrungszeugnis_auststellung DATE NOT NULL,
	fuehrungszeugnis_eingesehen DATE
);
