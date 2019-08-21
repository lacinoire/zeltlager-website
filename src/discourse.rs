use actix::prelude::*;
use rand::Rng;
use reqwest::multipart::Form;

use crate::config::DiscourseConfig;
use crate::db::models::Teilnehmer;
use crate::Result;

#[derive(Deserialize, Debug, Clone)]
struct UserCreateResponse {
	user_id: usize,
}

#[derive(Deserialize, Debug, Clone)]
struct GetGroupResponse { group: GroupResponse }
#[derive(Deserialize, Debug, Clone)]
struct GroupResponse { id: usize }

#[derive(Deserialize, Debug, Clone)]
struct GetCategoriesResponse { category_list: CategoryList }
#[derive(Deserialize, Debug, Clone)]
struct CategoryList { categories: Vec<Category> }
#[derive(Deserialize, Debug, Clone)]
struct Category { id: usize, name: String }

#[derive(Deserialize, Debug, Clone)]
struct GetUserResponse { user: UserResponse }
#[derive(Deserialize, Debug, Clone)]
struct UserResponse { id: usize }
#[derive(Deserialize, Debug, Clone)]
struct SignupResponse { user_id: usize }

pub struct DiscourseExecutor {
	config: DiscourseConfig,
	group_id: usize,
	category_id: usize,
}

impl Actor for DiscourseExecutor {
	type Context = SyncContext<Self>;
}

pub struct SignupMessage {
	pub member: Teilnehmer,
}

impl Message for SignupMessage {
	type Result = Result<()>;
}

impl DiscourseExecutor {
	pub fn new(config: DiscourseConfig) -> Result<Self> {
		let mut res = Self { config, group_id: 0, category_id: 0 };
		// Find group id
		let mut response = res.request_get(&format!("groups/{}", res.config.group))?;
		let response: GetGroupResponse = response.json()?;
		res.group_id = response.group.id;

		// Find category id
		let mut response = res.request_get("categories")?;
		let response: GetCategoriesResponse = response.json()?;
		res.category_id = response.category_list.categories.iter().filter(|c|
			c.name == res.config.category).next()
			.ok_or_else(|| format_err!("Cannot find category {:?} in {:?}",
				res.config.category, response.category_list.categories))?
			.id;
		Ok(res)
	}

	fn client(&self) -> reqwest::Client {
		reqwest::ClientBuilder::new()
			.build().unwrap()
	}

	fn request_get(&self, endpoint: &str) -> Result<reqwest::Response> {
		let client = self.client();
		Ok(client.get(&format!("{}/{}?api_key={}&api_username={}",
			self.config.endpoint, endpoint, self.config.token,
			self.config.username))
			.header("Accept", "application/json")
			.send()?)
	}

	fn request_post(&self, endpoint: &str, body: Form) -> Result<reqwest::Response> {
		let client = self.client();
		Ok(client.post(&format!("{}/{}",
			self.config.endpoint, endpoint))
			.multipart(body)
			.header("Accept", "application/json")
			.send()?)
	}

	fn request_put_without_content(&self, endpoint: &str) -> Result<reqwest::Response> {
		let client = self.client();
		Ok(client.put(&format!("{}/{}",
			self.config.endpoint, endpoint))
			.header("Accept", "application/json")
			.send()?)
	}

	fn request_put(&self, endpoint: &str, body: Form) -> Result<reqwest::Response> {
		let client = self.client();
		Ok(client.put(&format!("{}/{}",
			self.config.endpoint, endpoint))
			.multipart(body)
			.header("Accept", "application/json")
			.send()?)
	}
}

impl Handler<SignupMessage> for DiscourseExecutor {
	type Result = Result<()>;

	fn handle(
		&mut self,
		msg: SignupMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		let mut username: String = msg.member.eltern_name.chars()
			.filter(char::is_ascii_alphabetic)
			.collect();
		if username.len() < 3 {
			username.push_str("user");
		}

		let user_id;
		// Check if the user already exists
		let mut res = self.request_get(&format!("users/{}", username))?;
		// Store user_id and skip creating the user
		if let Ok(res) = res.json::<GetUserResponse>() {
			user_id = res.user.id;
		} else {
			// Create new user
			let pass = rand::thread_rng().gen::<usize>();
			let add = Form::new()
				.text("api_key", self.config.token.clone())
				.text("api_username", self.config.username.clone())

				.text("name", msg.member.eltern_name.clone())
				.text("username", username.clone())
				.text("email", msg.member.eltern_mail.clone())
				.text("password", pass.to_string())
				.text("active", "true")
				.text("approved", "true");
			let mut res = self.request_post("users", add)?;
			// Get user_id
			if !res.status().is_success() {
				let text = res.text();
				return Err(format_err!("Failed to add new user to discourse: {:?}, {:?}",
					res, text).into());
			}
			let res: SignupResponse = res.json()?;
			user_id = res.user_id;

			// Deactivate and activate again, otherwise the account is waiting for
			// the non-existing mail to be confirmed.
			let mut res = self.request_put_without_content(
				&format!("admin/users/{}/deactivate?api_key={}&api_username={}",
					user_id, self.config.token, self.config.username))?;
			if !res.status().is_success() {
				let text = res.text();
				return Err(format_err!("Failed to deactivate user: {:?}, {:?}",
					res, text).into());
			}

			// Activate mailing list mode
			/*
				email_always	false
				mailing_list_mode	true
				mailing_list_mode_frequency	2
				email_digests	true
				email_direct	true
				email_in_reply_to	true
				email_private_messages	true
				email_previous_replies	2
				digest_after_minutes	1440
				include_tl0_in_digests	true
			 */
			let form = Form::new()
				.text("api_key", self.config.token.clone())
				.text("api_username", self.config.username.clone())
				.text("mailing_list_mode", "true")
				.text("mailing_list_mode_frequency", "2");
			let mut res = self.request_put(&format!("u/{}", username), form)?;
			if !res.status().is_success() {
				let text = res.text();
				return Err(format_err!("Failed to set mailing list mode: {:?}, {:?}",
					res, text).into());
			}
		}

		let mut res = self.request_put_without_content(
			&format!("admin/users/{}/activate?api_key={}&api_username={}",
				user_id, self.config.token, self.config.username))?;
		if !res.status().is_success() {
			let text = res.text();
			return Err(format_err!("Failed to activate user: {:?}, {:?}",
				res, text).into());
		}


		// Add to group
		let form = Form::new()
			.text("api_key", self.config.token.clone())
			.text("api_username", self.config.username.clone())
			.text("usernames", username.clone());
		let mut res = self.request_put(&format!("groups/{}/members", self.group_id), form)?;
		if !res.status().is_success() {
			let text = res.text();
			return Err(format_err!("Failed to add new user to discourse group: {:?}, {:?}",
				res, text).into());
		}

		// Subscribe to category
		// Level 2 = Verfolgen
		let mut res = self.request_post(&format!("category/{}/notifications", self.category_id),
			Form::new()
			.text("api_key", self.config.token.clone())
			.text("api_username", username.clone())
			.text("notification_level", "2")
		)?;
		if !res.status().is_success() {
			let text = res.text();
			return Err(format_err!("Failed to subscribe new user to category: {:?}, {:?}",
				res, text).into());
		}

		Ok(())
	}
}
