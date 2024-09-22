-- Comments
ALTER TABLE teilnehmer
	ADD COLUMN krankheiten TEXT NOT NULL DEFAULT '';
ALTER TABLE teilnehmer
	RENAME COLUMN besonderheiten TO kommentar;
-- Add vegan as option
ALTER TABLE teilnehmer
	ADD COLUMN ernaehrung TEXT NOT NULL DEFAULT '';
UPDATE teilnehmer
    SET ernaehrung = case vegetarier when false then 'alles' else 'vegetarisch' end;
ALTER TABLE teilnehmer
	DROP COLUMN vegetarier;
-- Add anreise
ALTER TABLE teilnehmer
	ADD COLUMN eigenanreise BOOLEAN NOT NULL DEFAULT FALSE;

-- Comments
ALTER TABLE betreuer
	ADD COLUMN krankheiten TEXT;
ALTER TABLE betreuer
	RENAME COLUMN besonderheiten TO kommentar;
-- Add vegan as option
ALTER TABLE betreuer
	ADD COLUMN ernaehrung TEXT;
UPDATE betreuer
    SET ernaehrung = case vegetarier when false then 'alles' else 'vegetarisch' end;
ALTER TABLE betreuer
	DROP COLUMN vegetarier;

-- Juleica valid
ALTER TABLE betreuer
	ADD COLUMN juleica_gueltig_bis DATE;
-- Make more things optional for pre-signup
ALTER TABLE betreuer
	ALTER COLUMN strasse DROP NOT NULL,
	ALTER COLUMN hausnummer DROP NOT NULL,
	ALTER COLUMN ort DROP NOT NULL,
	ALTER COLUMN plz DROP NOT NULL,
	ALTER COLUMN kommentar DROP NOT NULL,
	ALTER COLUMN allergien DROP NOT NULL,
	ALTER COLUMN unvertraeglichkeiten DROP NOT NULL,
	ALTER COLUMN medikamente DROP NOT NULL,
	ALTER COLUMN krankenversicherung DROP NOT NULL,
	ALTER COLUMN tetanus_impfung DROP NOT NULL,
	ALTER COLUMN land DROP NOT NULL;
-- Re-signup token
ALTER TABLE betreuer
	ADD COLUMN signup_token DATE,
	ADD COLUMN signup_token_time TIMESTAMPTZ;
