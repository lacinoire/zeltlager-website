-- Your SQL goes here
ALTER TABLE betreuer
	ALTER COLUMN juleica_nummer DROP NOT NULL,
	ALTER COLUMN fuehrungszeugnis_auststellung DROP NOT NULL;
