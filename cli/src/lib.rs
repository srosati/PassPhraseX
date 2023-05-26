mod api;
mod file;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use app_dirs2::{app_dir, AppDataType, AppInfo};
use common::{KeyPair, SeedPhrase, EncryptedValue};
use api::Api;
use std::string::String;
use crate::file::{read_sk, write_sk};

pub const APP_INFO: AppInfo = AppInfo{name: "PassPhraseX", author: "Santos Matías Rosati"};

pub struct App<> {
    key_pair: KeyPair,
    // HashMap<site, HashMap<username, password>>
    credentials: HashMap<String, HashMap<String, EncryptedValue>>,
    api: Api
}

pub async fn register(device_pass: &str) -> SeedPhrase {
    let seed_phrase = SeedPhrase::new();
    let key_pair = KeyPair::new(seed_phrase.clone());

    let api = Api::new("http://localhost:3000");

    write_sk(key_pair.private_key.as_bytes())
        .expect("Failed to save private key to file");

    api.create_user(key_pair.get_pk()).await
        .expect("Failed to create user");

    seed_phrase
}

pub fn auth_device(seed_phrase: &str, device_pass: &str) {
    let seed_phrase = SeedPhrase::from_str(seed_phrase);
    let key_pair = KeyPair::new(seed_phrase.clone());

    let path_to_file = match app_dir(AppDataType::UserData, &APP_INFO, "data") {
        Ok(path) => path.join("private_key"),
        Err(e) => panic!("Error: {}", e)
    };

    match File::create(path_to_file) {
        Ok(mut file) => {
            match file.write_all(key_pair.private_key.as_bytes()) {
                Ok(_) => println!("Successfully authenticated device!"),
                Err(e) => panic!("Error: {}", e)
            }
        },
        Err(e) => panic!("Error: {}", e)
    }
}

impl App {
    pub fn new(device_pass: &str) -> App {
        let private_key = read_sk(device_pass).expect("Failed to read private key from file");
        let key_pair = KeyPair::from_sk(private_key);

        App {
            key_pair,
            credentials: HashMap::new(),
            api: Api::new("http://localhost:3000")
        }
    }

    pub async fn add(&mut self, site: String, username: String, password: String) -> Result<(), Box<dyn Error>>{
        let public_key = self.key_pair.get_pk();
        let username_enc = self.key_pair.encrypt(&username);
        let password_enc = self.key_pair.encrypt(&password);

        self.api.add_password(public_key, site, username_enc.into(), password_enc.into()).await?;

        Ok(())
    }

    pub async fn get(self, site: String, username: Option<String>) {
        let passwords = self.api.get_passwords(self.key_pair.get_pk(), site, username).await
            .expect("Failed to get password");

        for credential in passwords {

            let password_enc = EncryptedValue::from(credential.password);
            let username_enc = EncryptedValue::from(credential.username);

            let password_dec = self.key_pair.decrypt(&password_enc);
            let username_dec = self.key_pair.decrypt(&username_enc);

            println!("{}: {}", username_dec, password_dec);
        }
    }
}