-- Your SQL goes here
CREATE TABLE users (
	id SERIAL PRIMARY KEY,
	username TEXT NOT NULL,
	password TEXT NOT NULL
);

CREATE TABLE roles (
	id SERIAL PRIMARY KEY,
	role TEXT NOT NULL,
	user_id SERIAL NOT NULL REFERENCES users (id)
);

CREATE TABLE rate_limiting (
	ip_addr INET NOT NULL PRIMARY KEY,
	counter INT NOT NULL,
	first_count TIMESTAMP NOT NULL
);
