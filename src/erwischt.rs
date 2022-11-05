//! Erwischt game.

use actix_web::*;
use anyhow::bail;
use chrono::Utc;
use diesel::prelude::*;
use log::error;
use rand::seq::SliceRandom;
use serde::Deserialize;

use crate::{db, State};

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

#[get("/games")]
pub async fn get_games(state: web::Data<State>) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(|db| {
			use db::schema::erwischt_game;

			Ok(erwischt_game::table
				.order(erwischt_game::columns::created)
				.get_results::<db::models::ErwischtGame>(&mut db.connection)?)
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to get games: {}", e);
			HttpResponse::InternalServerError().body("Failed to get games")
		}
		Ok(Ok(r)) => HttpResponse::Ok().json(&r),
	}
}

#[get("/game/{id}")]
pub async fn get_game(state: web::Data<State>, game_id: web::Path<i32>) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::erwischt_member;
			use db::schema::erwischt_member::columns::*;

			Ok(erwischt_member::table
				.filter(game.eq(*game_id))
				.select((id, name, target, catcher, last_change))
				.order(id)
				.get_results::<db::models::ErwischtMember>(&mut db.connection)?)
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to get members: {}", e);
			HttpResponse::InternalServerError().body("Failed to get members")
		}
		Ok(Ok(r)) => HttpResponse::Ok().json(&r),
	}
}

#[post("/game")]
pub async fn create_game(state: web::Data<State>) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::betreuer;
			use db::schema::erwischt_game;
			use db::schema::erwischt_member;
			use db::schema::teilnehmer;

			let new_game: db::models::ErwischtGame = diesel::insert_into(erwischt_game::table)
				.default_values()
				.get_result(&mut db.connection)?;

			let mut member = Vec::new();

			let teilnehmer_member = teilnehmer::table
				.select((teilnehmer::columns::vorname, teilnehmer::columns::nachname))
				.get_results::<(String, String)>(&mut db.connection)?;

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
				.get_results::<(String, String)>(&mut db.connection)?;

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
			member.shuffle(&mut rand::thread_rng());
			let len = member.len();
			for i in 0..(len - 1) {
				member[i].target = member[i + 1].id;
			}
			member[len - 1].target = member[0].id;

			diesel::insert_into(erwischt_member::table)
				.values(member)
				.execute(&mut db.connection)?;

			Ok(new_game.id)
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to create game: {}", e);
			HttpResponse::InternalServerError().body("Failed to create game")
		}
		Ok(Ok(r)) => HttpResponse::Ok().json(r),
	}
}

#[delete("/game/{id}")]
pub async fn delete_game(state: web::Data<State>, game_id: web::Path<i32>) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::erwischt_game;
			use db::schema::erwischt_member;
			use db::schema::erwischt_member::columns::*;

			diesel::delete(erwischt_member::table)
				.filter(game.eq(*game_id))
				.execute(&mut db.connection)?;
			let r = diesel::delete(erwischt_game::table)
				.filter(erwischt_game::columns::id.eq(*game_id))
				.execute(&mut db.connection)?;

			if r == 0 {
				bail!("Game not found");
			}

			Ok(())
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to delete game: {}", e);
			HttpResponse::InternalServerError().body("Failed to delete game")
		}
		Ok(Ok(())) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

#[post("/game/setCatch")]
pub(crate) async fn catch(state: web::Data<State>, data: web::Json<CatchData>) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::erwischt_member;
			use db::schema::erwischt_member::columns::*;

			let r = diesel::update(erwischt_member::table)
				.filter(game.eq(data.game).and(id.eq(data.member)))
				.set((catcher.eq(data.catcher), last_change.eq(Some(Utc::now().naive_utc()))))
				.execute(&mut db.connection)?;
			if r == 0 {
				bail!("Member not found");
			}
			Ok(())
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to catch member: {}", e);
			HttpResponse::InternalServerError().body("Failed to catch member")
		}
		Ok(Ok(())) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

#[post("/game/insert")]
pub(crate) async fn insert(state: web::Data<State>, data: web::Json<InsertData>) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use diesel::dsl::max;

			use db::models::NewErwischtMember;
			use db::schema::erwischt_member;
			use db::schema::erwischt_member::columns::*;

			// Result will be `after` `new` `before`
			db.connection.transaction::<_, diesel::result::Error, _>(|con| {
				// Find current member before `before`
				let after = erwischt_member::table
					.filter(game.eq(data.game).and(target.eq(data.before)))
					.select(id)
					.get_result::<i32>(con)?;

				let last_id = erwischt_member::table
					.filter(game.eq(data.game))
					.select(max(id))
					.first::<Option<i32>>(con)?
					.ok_or(diesel::result::Error::NotFound)?;

				diesel::insert_into(erwischt_member::table)
					.values(NewErwischtMember {
						game: data.game,
						id: last_id + 1,
						name: data.name.clone(),
						target: data.before,
					})
					.execute(con)?;

				diesel::update(erwischt_member::table)
					.filter(game.eq(data.game).and(id.eq(after)))
					.set(target.eq(last_id + 1))
					.execute(con)?;

				Ok(())
			})?;

			Ok(())
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to insert member: {}", e);
			HttpResponse::InternalServerError().body("Failed to insert member")
		}
		Ok(Ok(())) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

#[get("/game/{id}/game.pdf")]
pub async fn create_game_pdf(state: web::Data<State>, game_id: web::Path<i32>) -> HttpResponse {
	create_pdf(&state, *game_id, true).await
}

#[get("/game/{id}/members.pdf")]
pub async fn create_members_pdf(state: web::Data<State>, game_id: web::Path<i32>) -> HttpResponse {
	create_pdf(&state, *game_id, false).await
}

async fn create_pdf(state: &State, game_id: i32, with_target: bool) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use std::collections::HashMap;
			use std::fs::File;
			use std::io::BufWriter;

			use diesel::dsl::max;
			use printpdf::*;

			use db::schema::erwischt_member;
			use db::schema::erwischt_member::columns::*;

			let last_id = erwischt_member::table
				.filter(game.eq(game_id))
				.select(max(id))
				.first::<Option<i32>>(&mut db.connection)?
				.ok_or(diesel::result::Error::NotFound)?;
			let id_digits = (last_id as f32).log10().ceil() as usize;

			let members = erwischt_member::table
				.filter(game.eq(game_id))
				.select((id, name, target, catcher, last_change))
				.get_results::<db::models::ErwischtMember>(&mut db.connection)?;
			let members: HashMap<i32, db::models::ErwischtMember> =
				members.into_iter().map(|m| (m.id, m)).collect();
			let mut members_by_name: Vec<i32> = members.keys().cloned().collect();
			members_by_name.sort_by_key(|i| &members[i].name);

			let width = Mm(210.0);
			let height = Mm(297.0);
			let (mut doc, mut page, mut layer) =
				PdfDocument::new("Erwischt", width, height, "Layer 1");
			// Remove ICC profile to reduce size
			doc = doc.with_conformance(PdfConformance::Custom(CustomPdfConformance {
				requires_icc_profile: false,
				requires_xmp_metadata: false,
				..Default::default()
			}));
			let font = doc.add_external_font(File::open("static/DejaVuSans.ttf")?)?;
			let mut current_layer = doc.get_page(page).get_layer(layer);

			let font_size = 11.0;
			let margin = Mm(12.0);
			let number_size = Mm(18.0);
			let line_height = Mm(7.0);
			let mut cur_y_pos = Mm(297.0) - margin;
			let mut cur_x_pos = margin;
			let mut column = 0;
			let mut page_num = 1;
			for (index, i) in members_by_name.iter().enumerate() {
				if column == 3 {
					// New page
					page_num += 1;
					column = 0;
					let (r_page, r_layer) =
						doc.add_page(width, height, &format!("Page {}, Layer 1", page_num));
					page = r_page;
					layer = r_layer;
					current_layer = doc.get_page(page).get_layer(layer);
					cur_y_pos = Mm(297.0) - margin;
					cur_x_pos = margin;
				}

				let m = &members[i];
				let target_index = members_by_name
					.iter()
					.position(|i| *i == m.target)
					.ok_or(diesel::result::Error::NotFound)?;
				if with_target {
					let text = format!("{:>width$}→{}", index, target_index, width = id_digits);
					current_layer.use_text(text, font_size, cur_x_pos, cur_y_pos, &font);
					let text = if m.name.len() > 20 {
						format!("{}.", &m.name[..19])
					} else {
						m.name.clone()
					};
					current_layer.use_text(
						text,
						font_size,
						cur_x_pos + number_size,
						cur_y_pos,
						&font,
					);
				} else {
					let text = m.name.clone();
					current_layer.use_text(text, font_size, cur_x_pos, cur_y_pos, &font);
				}
				cur_y_pos -= line_height;
				if cur_y_pos < margin {
					if column == 0 {
						cur_y_pos = Mm(297.0) - margin;
						cur_x_pos = width / 3.0;
					} else if column == 1 {
						cur_y_pos = Mm(297.0) - margin;
						cur_x_pos = width / 3.0 * 2.0;
					}
					column += 1;
				}
			}

			let mut buffer = Vec::new();
			doc.save(&mut BufWriter::new(&mut buffer))?;

			Ok(buffer)
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to get members: {}", e);
			HttpResponse::InternalServerError().body("Failed to get members")
		}
		Ok(Ok(r)) => HttpResponse::Ok().content_type("application/pdf").body(r),
	}
}
