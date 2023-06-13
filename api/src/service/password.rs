use mongodb::{Client, Collection};
use mongodb::bson::doc;
use mongodb::options::FindOneAndUpdateOptions;
use passphrasex_common::model::user::User;
use passphrasex_common::model::password::Password;
use crate::model::common::GetCollection;

#[derive(Clone)]
pub struct PasswordService {
	user_collection: Collection<User>
}

impl PasswordService {
	pub fn new(client: &Client) -> Self {
		Self {
			user_collection: client.get_collection("users")
		}
	}

	pub async fn list_passwords(&self, user_id: String) -> Result<Vec<Password>, String> {
		let filter = doc!{"_id": user_id.clone()};

		match self.user_collection.find_one(filter, None).await {
		    Ok(result) => match result {
		        Some(user) => Ok(user.passwords),
		        None => Err(format!("User with id {} not found", user_id))
		    },
		    Err(err) => Err(format!("Error getting passwords: {}", err.to_string()))
		}
	}

	pub async fn add_password(&self, user_id: String, password: Password) -> Result<Password, String> {
		let filter = doc!{"_id": user_id.clone()};

		let update = doc!{
			"$addToSet": {
				"passwords": {
					"_id": password._id.clone(),
					"site": password.site.clone(),
					"username": password.username.clone(),
					"password": password.password.clone()
				}
			}
		};

		match self.user_collection.find_one_and_update(filter, update, None).await {
		    Ok(result) => match result {
		        Some(_) => Ok(password),
		        None => Err(format!("User with id {} not found", user_id))
		    },
		    Err(err) => Err(err.to_string())
		}
	}

	pub async fn delete_password(&self, user_id: String, password_id: String) -> Result<(), String> {
		let filter = doc!{"_id": user_id.clone()};

		let update = doc!{
			"$pull": {
				"passwords": {
					"_id": password_id.clone()
				}
			}
		};

		match self.user_collection.find_one_and_update(filter, update, None).await {
		    Ok(result) => match result {
		        Some(_) => Ok(()),
		        None => Err(format!("User with id {} not found", user_id))
		    },
		    Err(err) => Err(err.to_string())
		}
	}

	pub async fn modify_password(&self, user_id: String, password_id: String, password: String) -> Result<(), String> {
		let filter = doc!{"_id": user_id.clone()};

		let update = doc!{
			"$set": {
				"passwords.$[password].password": password.clone()
			}
		};

		let options = FindOneAndUpdateOptions::builder()
			.array_filters(Some(vec![doc!{"password._id": password_id.clone()}]))
			.build();

		// let array_filters = doc!{
		// 	"arrayFilters": [{
		// 		"password._id": password_id.clone()
		// 	}]
		// };

		match self.user_collection.find_one_and_update(filter, update, Some(options)).await {
		    Ok(result) => match result {
		        Some(_) => Ok(()),
		        None => Err(format!("User with id {} not found", user_id))
		    },
		    Err(err) => Err(err.to_string())
		}
	}
}

