-- Your SQL goes here
CREATE TABLE erwischt_game (
	id SERIAL PRIMARY KEY,
	created TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE erwischt_member (
	game SERIAL NOT NULL REFERENCES erwischt_game (id),
	id SERIAL NOT NULL,
	name TEXT NOT NULL,
	target SERIAL NOT NULL,
	catcher SERIAL,
	last_change TIMESTAMPTZ,

	PRIMARY KEY (game, id),
	FOREIGN KEY (game, target) REFERENCES erwischt_member (game, id),
	FOREIGN KEY (game, catcher) REFERENCES erwischt_member (game, id)
);
