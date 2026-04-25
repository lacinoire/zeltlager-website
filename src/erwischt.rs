//! Erwischt game.

use anyhow::{Error, bail};
use axum::{
	Json,
	extract::{self, Path},
	http::StatusCode,
	response::IntoResponse,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use rand::seq::SliceRandom;
use serde::Deserialize;
use time::{OffsetDateTime, PrimitiveDateTime};
use tracing::error;

use crate::{ExtractState, WebResult, db};

type DbResult<T> = anyhow::Result<T>;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CatchData {
	game: i32,
	catcher: Option<i32>,
	member: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct InsertData {
	game: i32,
	before: i32,
	name: String,
}

fn err<T>(error: Error, msg: &'static str) -> WebResult<T> {
	error!(%error, "{msg}");
	Err((StatusCode::INTERNAL_SERVER_ERROR, msg).into_response())
}

pub async fn get_games(
	extract::State(state): ExtractState,
) -> WebResult<Json<Vec<db::models::ErwischtGame>>> {
	match async {
		use db::schema::erwischt_game;

		DbResult::Ok(
			erwischt_game::table
				.order(erwischt_game::columns::created)
				.get_results::<db::models::ErwischtGame>(&mut state.db.get().await?)
				.await?,
		)
	}
	.await
	{
		Err(error) => err(error, "Failed to get games"),
		Ok(r) => Ok(Json(r)),
	}
}

pub async fn get_game(
	extract::State(state): ExtractState, Path(game_id): Path<i32>,
) -> WebResult<Json<Vec<db::models::ErwischtMember>>> {
	match async {
		use db::schema::erwischt_member;
		use db::schema::erwischt_member::columns::*;

		DbResult::Ok(
			erwischt_member::table
				.filter(game.eq(game_id))
				.select((id, name, target, catcher, last_change))
				.order(id)
				.get_results::<db::models::ErwischtMember>(&mut state.db.get().await?)
				.await?,
		)
	}
	.await
	{
		Err(error) => err(error, "Failed to get game members"),
		Ok(r) => Ok(Json(r)),
	}
}

pub async fn create_game(extract::State(state): ExtractState) -> WebResult<Json<i32>> {
	match async {
		use db::schema::betreuer;
		use db::schema::erwischt_game;
		use db::schema::erwischt_member;
		use db::schema::teilnehmer;

		let mut connection = state.db.get().await?;

		let new_game: db::models::ErwischtGame = diesel::insert_into(erwischt_game::table)
			.default_values()
			.get_result(&mut connection)
			.await?;

		let mut member = Vec::new();

		let teilnehmer_member = teilnehmer::table
			.select((teilnehmer::columns::vorname, teilnehmer::columns::nachname))
			.get_results::<(String, String)>(&mut connection)
			.await?;

		for m in teilnehmer_member {
			let m_id = member.len() as i32;
			member.push(db::models::NewErwischtMember {
				game: new_game.id,
				id: m_id,
				name: format!("{} {}", m.0, m.1),
				target: 0,
			});
		}

		let supervisor_member = betreuer::table
			.select((betreuer::columns::vorname, betreuer::columns::nachname))
			.get_results::<(String, String)>(&mut connection)
			.await?;

		for m in supervisor_member {
			let m_id = member.len() as i32;
			member.push(db::models::NewErwischtMember {
				game: new_game.id,
				id: m_id,
				name: format!("{} {}", m.0, m.1),
				target: 0,
			});
		}

		if member.is_empty() {
			bail!("Cannot create an empty game");
		}

		// Set targets of members
		member.shuffle(&mut rand::rng());
		let len = member.len();
		for i in 0..(len - 1) {
			member[i].target = member[i + 1].id;
		}
		member[len - 1].target = member[0].id;

		diesel::insert_into(erwischt_member::table).values(member).execute(&mut connection).await?;

		Ok(new_game.id)
	}
	.await
	{
		Err(error) => err(error, "Failed to create game"),
		Ok(r) => Ok(Json(r)),
	}
}

pub async fn delete_game(
	extract::State(state): ExtractState, Path(game_id): Path<i32>,
) -> WebResult<&'static str> {
	match async {
		use db::schema::erwischt_game;
		use db::schema::erwischt_member;
		use db::schema::erwischt_member::columns::*;

		let mut connection = state.db.get().await?;

		diesel::delete(erwischt_member::table)
			.filter(game.eq(game_id))
			.execute(&mut connection)
			.await?;
		let r = diesel::delete(erwischt_game::table)
			.filter(erwischt_game::columns::id.eq(game_id))
			.execute(&mut connection)
			.await?;

		if r == 0 {
			bail!("Game not found");
		}

		Ok(())
	}
	.await
	{
		Err(error) => err(error, "Failed to delete game"),
		Ok(()) => Ok("Success"),
	}
}

pub(crate) async fn catch(
	extract::State(state): ExtractState, Json(data): Json<CatchData>,
) -> WebResult<&'static str> {
	match async {
		use db::schema::erwischt_member;
		use db::schema::erwischt_member::columns::*;

		let now = OffsetDateTime::now_utc();
		let r = diesel::update(erwischt_member::table)
			.filter(game.eq(data.game).and(id.eq(data.member)))
			.set((
				catcher.eq(data.catcher),
				last_change.eq(Some(PrimitiveDateTime::new(now.date(), now.time()))),
			))
			.execute(&mut state.db.get().await?)
			.await?;
		if r == 0 {
			bail!("Member not found");
		}
		Ok(())
	}
	.await
	{
		Err(error) => err(error, "Failed to catch member"),
		Ok(()) => Ok("Success"),
	}
}

pub(crate) async fn insert(
	extract::State(state): ExtractState, Json(data): Json<InsertData>,
) -> WebResult<&'static str> {
	match async {
		use diesel::dsl::max;

		use db::models::NewErwischtMember;
		use db::schema::erwischt_member;
		use db::schema::erwischt_member::columns::*;

		// Result will be `after` `new` `before`
		state
			.db
			.get()
			.await?
			.transaction::<_, diesel::result::Error, _>(|con| {
				async move {
					// Find current member before `before`
					let after = erwischt_member::table
						.filter(game.eq(data.game).and(target.eq(data.before)))
						.select(id)
						.get_result::<i32>(con)
						.await?;

					let last_id = erwischt_member::table
						.filter(game.eq(data.game))
						.select(max(id))
						.first::<Option<i32>>(con)
						.await?
						.ok_or(diesel::result::Error::NotFound)?;

					diesel::insert_into(erwischt_member::table)
						.values(NewErwischtMember {
							game: data.game,
							id: last_id + 1,
							name: data.name.clone(),
							target: data.before,
						})
						.execute(con)
						.await?;

					diesel::update(erwischt_member::table)
						.filter(game.eq(data.game).and(id.eq(after)))
						.set(target.eq(last_id + 1))
						.execute(con)
						.await?;

					Ok(())
				}
				.scope_boxed()
			})
			.await?;
		DbResult::Ok(())
	}
	.await
	{
		Err(error) => err(error, "Failed to insert member"),
		Ok(()) => Ok("Success"),
	}
}
