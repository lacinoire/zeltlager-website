ALTER TABLE teilnehmer
	DROP COLUMN krankheiten;
ALTER TABLE teilnehmer
	RENAME COLUMN kommentar TO besonderheiten;
ALTER TABLE teilnehmer
	ADD COLUMN vegetarier BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE teilnehmer
    SET vegetarier = (ernaehrung = 'vegetarisch');
ALTER TABLE teilnehmer
	DROP COLUMN ernaehrung;
ALTER TABLE teilnehmer
	DROP COLUMN eigenanreise;

ALTER TABLE betreuer
	ALTER COLUMN strasse SET NOT NULL,
	ALTER COLUMN hausnummer SET NOT NULL,
	ALTER COLUMN ort SET NOT NULL,
	ALTER COLUMN plz SET NOT NULL,
	ALTER COLUMN kommentar SET NOT NULL,
	ALTER COLUMN allergien SET NOT NULL,
	ALTER COLUMN unvertraeglichkeiten SET NOT NULL,
	ALTER COLUMN medikamente SET NOT NULL,
	ALTER COLUMN krankenversicherung SET NOT NULL,
	ALTER COLUMN tetanus_impfung SET NOT NULL,
	ALTER COLUMN land SET NOT NULL;
ALTER TABLE betreuer
	DROP COLUMN juleica_gueltig_bis,
	DROP COLUMN signup_token,
	DROP COLUMN signup_token_time;

ALTER TABLE betreuer
	DROP COLUMN krankheiten;
ALTER TABLE betreuer
	RENAME COLUMN kommentar TO besonderheiten;
ALTER TABLE betreuer
	ADD COLUMN vegetarier BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE betreuer
    SET vegetarier = (ernaehrung = 'vegetarisch');
ALTER TABLE betreuer
	DROP COLUMN ernaehrung;
