-- Your SQL goes here
CREATE TABLE teilnehmer (
	id SERIAL PRIMARY KEY,
	vorname TEXT NOT NULL,
    nachname TEXT NOT NULL,
    geburtsdatum DATE NOT NULL,
    schwimmer BOOLEAN NOT NULL,
    vegetarier BOOLEAN NOT NULL,
    tetanus_impfung BOOLEAN NOT NULL,
    eltern_name TEXT NOT NULL,
    eltern_mail TEXT NOT NULL,
    eltern_handynummer TEXT NOT NULL,
    strasse TEXT NOT NULL,
    hausnummer TEXT NOT NULL,
    ort TEXT NOT NULL,
    plz TEXT NOT NULL,
    besonderheiten TEXT NOT NULL,
    agb BOOLEAN NOT NULL
);
