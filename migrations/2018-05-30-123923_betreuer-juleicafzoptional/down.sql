-- This file should undo anything in `up.sql`
ALTER TABLE betreuer
	ALTER COLUMN juleica_nummer SET NOT NULL,
	ALTER COLUMN fuehrungszeugnis_auststellung SET NOT NULL;
