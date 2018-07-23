-- Your SQL goes here
CREATE TABLE users (
	id SERIAL PRIMARY KEY,
	username TEXT NOT NULL UNIQUE,
	password TEXT NOT NULL
);

CREATE TABLE roles (
	user_id SERIAL NOT NULL REFERENCES users (id),
	role TEXT NOT NULL,
	PRIMARY KEY (user_id, role)
);

CREATE TABLE rate_limiting (
	ip_addr INET NOT NULL PRIMARY KEY,
	counter INT NOT NULL,
	first_count TIMESTAMP NOT NULL
);
