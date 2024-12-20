use std::io::Write;
use std::{env, io};

use anyhow::{Result, format_err};
use diesel::Connection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use scrypt::Scrypt;
use scrypt::password_hash::{PasswordHasher, SaltString};

use crate::config::Action;
use crate::db;

fn ask_username() -> String {
	print!("Enter username: ");
	io::stdout().flush().unwrap();
	let mut name = String::new();
	io::stdin().read_line(&mut name).unwrap();
	name.trim().to_string()
}

fn confirm(msg: &str) -> bool {
	print!("{} [y|n] ", msg);
	io::stdout().flush().unwrap();
	let mut name = String::new();
	io::stdin().read_line(&mut name).unwrap();
	["y", "Y", "yes", "Yes", "YES"].contains(&name.trim())
}

pub(crate) fn cmd_action(action: Action) -> Result<()> {
	use crate::db::schema::users::dsl::*;

	dotenv().ok();
	let database_url = env::var("DATABASE_URL").map_err(|e| {
		format_err!("DATABASE_URL is not set, are you missing a .env file? ({:?})", e)
	})?;
	let mut connection = PgConnection::establish(&database_url)?;
	match action {
		Action::AddUser { username: name, force } => {
			let name = name.unwrap_or_else(ask_username);
			let exists = diesel::select(diesel::dsl::exists(users.filter(username.eq(&name))))
				.get_result(&mut connection)?;
			// Check if the user exists
			// Ask for confirmation
			if !force
				&& exists && !confirm(&format!(
				"The user '{}' exists. Would you like to overwrite its password?",
				name
			)) {
				println!("Aborted by user");
				return Ok(());
			}

			let pw = rpassword::prompt_password("Please enter the password: ").unwrap();
			let salt = SaltString::generate(&mut rand::thread_rng());
			let pw = Scrypt.hash_password(pw.as_bytes(), &salt)?.to_string();
			if exists {
				diesel::update(users.filter(username.eq(&name)))
					.set(password.eq(pw))
					.execute(&mut connection)?;
			} else {
				let user = db::models::User { username: name, password: pw };
				diesel::insert_into(db::schema::users::table)
					.values(&user)
					.execute(&mut connection)?;
			}
		}
		Action::DelUser { username: name } => {
			let name = name.unwrap_or_else(ask_username);
			let count =
				diesel::delete(users.filter(username.eq(&name))).execute(&mut connection)?;
			if count == 0 {
				println!("User not found");
			} else {
				println!("Deleted {} user", count);
			}
		}
	}

	Ok(())
}
