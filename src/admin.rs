use std::io::ErrorKind;
use std::sync::Arc;
use std::{fs, mem};

use anyhow::{Error, Result, bail};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, extract};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::db::models::{FullSupervisor, FullTeilnehmer, User};
use crate::{ExtractState, State, WebResult, auth, db, mail, thumbs};
use time::OffsetDateTime;

type DbResult<T> = anyhow::Result<T>;

#[derive(Clone, Debug, Deserialize)]
pub struct RemoveMemberData {
	member: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RemoveSupervisorData {
	supervisor: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct EditMemberResult {
	error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LagerInfo {
	teilnehmer_count: i64,
	old_betreuer_count: i64,
	erwischt_game_count: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KanidmUser {
	attrs: KanidmUserAttrs,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KanidmUserAttrs {
	name: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KanidmPasswordReset {
	token: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct KanidmCreateUser {
	attrs: KanidmCreateUserAttrs,
}

#[derive(Clone, Debug, Serialize)]
pub struct KanidmCreateUserAttrs {
	name: Vec<String>,
	displayname: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserData {
	user: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ImageLinkData {
	name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImageLinkEntry {
	name: String,
	user: String,
	url: String,
}

fn err<T>(error: Error, msg: &'static str) -> WebResult<T> {
	error!(%error, "{msg}");
	Err((StatusCode::INTERNAL_SERVER_ERROR, msg).into_response())
}

pub(crate) async fn remove_member(
	extract::State(state): ExtractState, Json(data): Json<RemoveMemberData>,
) -> WebResult<&'static str> {
	match async {
		use db::schema::teilnehmer;
		use db::schema::teilnehmer::columns::*;

		let r = diesel::delete(teilnehmer::table.filter(id.eq(data.member)))
			.execute(&mut state.db.get().await?)
			.await?;
		if r == 0 {
			bail!("Member not found");
		}
		Ok(())
	}
	.await
	{
		Err(error) => err(error, "Failed to remove member"),
		Ok(()) => Ok("Success"),
	}
}

/// Write mail to confirm payment
async fn payed_mail(
	mail: &mail::Mail, member: db::models::FullTeilnehmer,
) -> (StatusCode, EditMemberResult) {
	let error = match mail.send_member_payed(&member).await {
		Err(error) => {
			error!(mail = member.eltern_mail, %error, "Error sending e-mail");
			format!(
				"Die Änderung wurde erfolgreich gespeichert.\nEs ist leider ein Fehler beim \
				 E-Mail senden aufgetreten.\n{}",
				error
			)
		}
		Ok(()) => {
			// Signup successful
			return (StatusCode::OK, EditMemberResult { error: None });
		}
	};

	(StatusCode::INTERNAL_SERVER_ERROR, EditMemberResult { error: Some(error) })
}

pub(crate) async fn edit_member(
	extract::State(state): ExtractState, Json(data): Json<FullTeilnehmer>,
) -> Response {
	match async {
		use db::schema::teilnehmer;
		use db::schema::teilnehmer::columns::*;

		let mut connection = state.db.get().await?;

		let member = teilnehmer::table
			.filter(id.eq(data.id))
			.get_result::<db::models::FullTeilnehmer>(&mut connection)
			.await?;
		let new_payed = data.bezahlt && !member.bezahlt;

		diesel::update(&data).set(&data).execute(&mut connection).await?;
		DbResult::Ok((new_payed, member))
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to edit member");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(EditMemberResult {
					error: Some(format!("Teilnehmer konnte nicht gespeichert werden: {error}")),
				}),
			)
				.into_response()
		}
		Ok((new_payed, member)) => {
			if new_payed {
				let (status, result) = payed_mail(&state.mail, member).await;
				(status, Json(result)).into_response()
			} else {
				Json(EditMemberResult { error: None }).into_response()
			}
		}
	}
}

// TODO Use delete("/betreuer/{id}") here and for teilnehmer
pub(crate) async fn remove_supervisor(
	extract::State(state): ExtractState, Json(data): Json<RemoveSupervisorData>,
) -> WebResult<&'static str> {
	match async {
		use db::schema::betreuer;
		use db::schema::betreuer::columns::*;

		let r = diesel::delete(betreuer::table.filter(id.eq(data.supervisor)))
			.execute(&mut state.db.get().await?)
			.await?;
		if r == 0 {
			bail!("Supervisor not found");
		}
		Ok(())
	}
	.await
	{
		Err(error) => err(error, "Failed to remove supervisor"),
		Ok(()) => Ok("Success"),
	}
}

pub(crate) async fn edit_supervisor(
	extract::State(state): ExtractState, Json(data): Json<FullSupervisor>,
) -> WebResult<&'static str> {
	match async {
		diesel::update(&data).set(&data).execute(&mut state.db.get().await?).await?;
		DbResult::Ok(())
	}
	.await
	{
		Err(error) => err(error, "Failed to edit supervisor"),
		Ok(()) => Ok("Success"),
	}
}

/// Return all current members as json.
pub async fn download_members(
	extract::State(state): ExtractState,
) -> WebResult<Json<Vec<FullTeilnehmer>>> {
	let mut connection = match state.db.get().await {
		Ok(c) => c,
		Err(error) => {
			warn!(%error, "Error getting database connection");
			return crate::error_response(&state);
		}
	};

	match db::schema::teilnehmer::table.load::<FullTeilnehmer>(&mut connection).await {
		Err(error) => {
			warn!(%error, "Error fetching from database");
			crate::error_response(&state)
		}
		Ok(members) => Ok(Json(members)),
	}
}

/// Return all supervisors as json.
pub async fn download_supervisors(
	extract::State(state): ExtractState,
) -> WebResult<Json<Vec<FullSupervisor>>> {
	let mut connection = match state.db.get().await {
		Ok(c) => c,
		Err(error) => {
			warn!(%error, "Error getting database connection");
			return crate::error_response(&state);
		}
	};

	match db::schema::betreuer::table.load::<FullSupervisor>(&mut connection).await {
		Err(error) => {
			warn!(%error, "Error fetching from database");
			crate::error_response(&state)
		}
		Ok(supervisors) => Ok(Json(supervisors)),
	}
}

pub async fn download_mails(
	extract::State(state): ExtractState,
) -> Result<Json<Vec<String>>, Response> {
	match async {
		use crate::db::schema::teilnehmer;

		let mut mails = teilnehmer::table
			.select(teilnehmer::eltern_mail)
			.load::<String>(&mut state.db.get().await?)
			.await?;
		mails.sort();
		mails.dedup();

		DbResult::Ok(mails)
	}
	.await
	{
		Err(error) => {
			warn!(%error, "Error fetching from database");
			crate::error_response(&state)
		}
		Ok(mails) => Ok(Json(mails)),
	}
}

/// The date at which betreuer need to be signed up to count towards the current or last year.
fn betreuer_signup_date_last_year() -> OffsetDateTime {
	let mut date = crate::LAGER_START.midnight().assume_utc();
	let now = OffsetDateTime::now_utc();
	if date > now {
		// Start from next time, subtract a year to allow signups from last time.
		date -= time::Duration::days(365);
	}

	// Add two weeks to get the end of the lager, subtract one year to get the date of last year.
	date - time::Duration::days(365) + time::Duration::weeks(2)
}

/// Get overview info of the current lager.
pub async fn lager_info(extract::State(state): ExtractState) -> WebResult<Json<LagerInfo>> {
	match async {
		use crate::db::schema::{betreuer, erwischt_game, teilnehmer};
		use diesel::dsl;

		let mut connection = state.db.get().await?;

		let teilnehmer_count = teilnehmer::table.count().get_result(&mut connection).await?;
		let old_betreuer_count = betreuer::table
			.filter(
				betreuer::anmeldedatum
					.lt(betreuer_signup_date_last_year())
					.or(dsl::not(betreuer::selbsterklaerung)),
			)
			.count()
			.get_result(&mut connection)
			.await?;
		let erwischt_game_count = erwischt_game::table.count().get_result(&mut connection).await?;

		DbResult::Ok(LagerInfo { teilnehmer_count, old_betreuer_count, erwischt_game_count })
	}
	.await
	{
		Err(error) => {
			warn!(%error, "Error getting lager info");
			crate::error_response(&state)
		}
		Ok(info) => Ok(Json(info)),
	}
}

/// Remove all data for the current lager.
pub async fn remove_lager(extract::State(state): ExtractState) -> WebResult<&'static str> {
	// Remove log file
	if let Some(log_file) = &state.config.log_file {
		if let Err(error) = std::fs::remove_file(log_file) {
			if error.kind() != std::io::ErrorKind::NotFound {
				error!(file = %log_file.display(), %error, "Failed to remove log file");
			}
		}
	}

	match async {
		use crate::db::schema::{betreuer, erwischt_game, erwischt_member, teilnehmer};
		use diesel::dsl;

		let mut connection = state.db.get().await?;

		diesel::delete(teilnehmer::table).execute(&mut connection).await?;
		diesel::delete(erwischt_member::table).execute(&mut connection).await?;
		diesel::delete(erwischt_game::table).execute(&mut connection).await?;
		diesel::delete(
			betreuer::table.filter(
				betreuer::anmeldedatum
					.lt(betreuer_signup_date_last_year())
					.or(dsl::not(betreuer::selbsterklaerung)),
			),
		)
		.execute(&mut connection)
		.await?;

		DbResult::Ok(())
	}
	.await
	{
		Err(error) => {
			warn!(%error, "Error deleting lager");
			crate::error_response(&state)
		}
		Ok(()) => Ok("Success"),
	}
}

pub async fn list_users_intern(state: &State) -> Result<Json<Vec<String>>> {
	let client = reqwest::Client::new();
	let r: Vec<KanidmUser> = client
		.get(format!("{}/v1/person", state.get_kanidm_base_url()?))
		.bearer_auth(state.get_kanidm_token()?)
		.send()
		.await?
		.json()
		.await?;
	Ok(Json(r.into_iter().map(|mut u| mem::take(&mut u.attrs.name[0])).collect()))
}

/// List admin users
pub async fn list_users(extract::State(state): ExtractState) -> WebResult<Json<Vec<String>>> {
	match list_users_intern(&state).await {
		Err(error) => err(error, "Failed to list users"),
		Ok(r) => Ok(r),
	}
}

pub async fn reset_password_intern(state: &State, user: &str) -> Result<String> {
	let client = reqwest::Client::new();
	let r: KanidmPasswordReset = client
		.get(format!(
			"{}/v1/person/{user}/_credential/_update_intent",
			state.get_kanidm_base_url()?
		))
		.bearer_auth(state.get_kanidm_token()?)
		.send()
		.await?
		.json()
		.await?;
	Ok(format!("{}/ui/reset?token={}", state.get_kanidm_base_url()?, r.token))
}

pub async fn reset_password(
	extract::State(state): ExtractState, Query(user): Query<UserData>,
) -> WebResult<String> {
	match reset_password_intern(&state, &user.user).await {
		Err(error) => err(error, "Failed to reset password"),
		Ok(r) => Ok(r),
	}
}

async fn create_user_intern(state: &State, user: &str) -> Result<()> {
	let client = reqwest::Client::new();
	client
		.post(format!("{}/v1/person", state.get_kanidm_base_url()?))
		.bearer_auth(state.get_kanidm_token()?)
		.json(&KanidmCreateUser {
			attrs: KanidmCreateUserAttrs {
				name: vec![user.to_lowercase()],
				displayname: vec![user.into()],
			},
		})
		.send()
		.await?;

	// Add to group
	client
		.post(format!("{}/v1/group/zeltlager/_attr/member", state.get_kanidm_base_url()?))
		.bearer_auth(state.get_kanidm_token()?)
		.json(&[user])
		.send()
		.await?;

	Ok(())
}

pub async fn create_user(
	extract::State(state): ExtractState, Query(user): Query<UserData>,
) -> WebResult<()> {
	match create_user_intern(&state, &user.user).await {
		Err(error) => err(error, "Failed to create user"),
		Ok(()) => Ok(()),
	}
}

async fn create_image_link_intern(state: &Arc<State>, name: &str) -> Result<()> {
	// Check name
	if !name.starts_with("Bilder") {
		bail!("Name muss mit 'Bilder' beginnen");
	}
	if !name.chars().all(|c| c.is_ascii_alphanumeric()) {
		bail!("Name darf nur Buchstaben und Zahlen beinhalten");
	}

	// 1. Create user if it does not exist
	let count: i64 = db::schema::users::dsl::users
		.filter(db::schema::users::username.eq(&name))
		.count()
		.get_result(&mut state.db.get().await?)
		.await?;
	if count == 0 {
		// Create without password
		let user = User { username: name.into(), password: String::new() };
		diesel::insert_into(db::schema::users::table)
			.values(&user)
			.execute(&mut state.db.get().await?)
			.await?;
	}

	// Get user id
	let user = db::schema::users::dsl::users
		.filter(db::schema::users::username.eq(name))
		.select(db::schema::users::id)
		.first::<i32>(&mut state.db.get().await?)
		.await?;

	let mut created_dir = false;
	if count == 0 {
		// Add role for new user
		let role = db::models::Role {
			user_id: user,
			role: format!("Images{}", name.strip_prefix("Bilder").unwrap()),
		};
		diesel::insert_into(db::schema::roles::table)
			.values(&role)
			.execute(&mut state.db.get().await?)
			.await?;

		// Create folder if it does not exist
		match fs::create_dir(name) {
			Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
			Err(e) => return Err(e.into()),
			Ok(()) => {
				// Folder was created
				// Start thumbnail watcher
				let state2 = state.clone();
				let name2 = name.into();
				std::thread::spawn(move || thumbs::watch_thumbs(&state2, name2));
				created_dir = true;
			}
		}
	}

	// Ensure the user has exactly one role and it is for images (to forbid token logins for admins)
	let roles = db::schema::roles::dsl::roles
		.filter(db::schema::roles::dsl::user_id.eq(user))
		.get_results::<db::models::Role>(&mut state.db.get().await?)
		.await?;
	if roles.len() != 1 {
		// TODO Improve user-facing errors
		bail!("Refuse to create auth token, user has not one role but {}", roles.len());
	}
	let role: auth::Roles = roles[0].role.parse()?;
	if !matches!(role, auth::Roles::Images(_)) {
		bail!("Refuse to create auth token, user role is not images but {:?}", role);
	}

	// Ensure that the user has no auth token set yet
	let existing_token = db::schema::users::dsl::users
		.filter(db::schema::users::id.eq(user))
		.select(db::schema::users::token)
		.first::<Option<String>>(&mut state.db.get().await?)
		.await?;
	if existing_token.is_some() {
		bail!("Refuse to create auth token, user {} already has an auth token set", name);
	}

	// Create and set auth token
	let token = {
		let mut rng = rand::thread_rng();
		(0..24).map(|_| rng.sample(rand::distributions::Alphanumeric) as char).collect::<String>()
	};
	diesel::update(db::schema::users::table)
		.filter(db::schema::users::dsl::id.eq(user))
		.set(db::schema::users::dsl::token.eq(Some(&token)))
		.execute(&mut state.db.get().await?)
		.await?;

	if created_dir {
		// Restart webserver
		state.recreate_webserver.send(()).await?;
	}

	Ok(())
}

async fn delete_image_link_intern(state: &State, name: &str) -> Result<()> {
	// Delete auth token
	let count = diesel::update(db::schema::users::table)
		.filter(db::schema::users::dsl::username.eq(name))
		.set(db::schema::users::dsl::token.eq(None::<String>))
		.execute(&mut state.db.get().await?)
		.await?;

	if count != 1 {
		bail!("Failed to remove auth token for user {name}");
	}

	Ok(())
}

async fn list_image_links_intern(state: &State) -> Result<Vec<ImageLinkEntry>> {
	// List users
	let users = db::schema::users::table
		.filter(db::schema::users::dsl::token.is_not_null())
		.get_results::<db::models::UserQueryResult>(&mut state.db.get().await?)
		.await?;

	let mut links = Vec::new();
	for u in users {
		links.push(ImageLinkEntry {
			name: u.username.clone(),
			user: u.username.clone(),
			url: format!("/{}/?login_token={}", u.username, u.token.unwrap()),
		})
	}

	Ok(links)
}

pub async fn create_image_link(
	extract::State(state): ExtractState, Query(link): Query<ImageLinkData>,
) -> WebResult<()> {
	match create_image_link_intern(&state, &link.name).await {
		Err(error) => err(error, "Failed to create image link"),
		Ok(()) => Ok(()),
	}
}

pub async fn delete_image_link(
	extract::State(state): ExtractState, Query(link): Query<ImageLinkData>,
) -> WebResult<()> {
	match delete_image_link_intern(&state, &link.name).await {
		Err(error) => err(error, "Failed to delete image link"),
		Ok(()) => Ok(()),
	}
}

pub async fn list_image_links(
	extract::State(state): ExtractState,
) -> WebResult<Json<Vec<ImageLinkEntry>>> {
	match list_image_links_intern(&state).await {
		Err(error) => err(error, "Failed to list image links"),
		Ok(r) => Ok(Json(r)),
	}
}
