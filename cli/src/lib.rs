mod api;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use app_dirs2::{app_dir, AppDataType, AppInfo};
use common::{KeyPair, SeedPhrase, EncryptedValue};
use api::Api;

const APP_INFO: AppInfo = AppInfo{name: "PassPhraseX", author: "Santos Rosati"};

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

    save_sk(key_pair.private_key.as_bytes())
        .expect("Failed to save private key to file");

    api.create_user(key_pair.get_pk()).await
        .expect("Failed to create user");

    seed_phrase
}

fn save_sk(public_key: &[u8; 32]) -> Result<(), Box<dyn Error>> {
    let path_to_file = app_dir(AppDataType::UserData, &APP_INFO, "data")?
        .join("secret_key");

    println!("Path: {:?}", path_to_file);
    let mut file = File::create(path_to_file)?;
    file.write_all(public_key)?;

    Ok(())
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
        let private_key = get_sk(device_pass);
        let key_pair = KeyPair::from_sk(private_key);

        App {
            key_pair,
            credentials: HashMap::new(),
            api: Api::new("http://localhost:3000")
        }
    }

    pub async fn add(&mut self, site: String, username: String, password: String) -> Result<(), Box<dyn Error>>{
        // let mut site_credentials: HashMap<String, EncryptedValue> = HashMap::new();
        // site_credentials.insert(
        //     username,
        //     self.key_pair.encrypt(&password)
        // );

        let public_key = self.key_pair.get_pk();
        let username = self.key_pair.encrypt(&username).cipher;
        let password = self.key_pair.encrypt(&password).cipher;

        self.api.add_password(public_key, site, username, password).await?;

        Ok(())



    }

    pub fn get(self, site: String, username: Option<String>) {
        let site_credentials = self.credentials.get(&site).unwrap();
        // TODO: Print all usernames if username is None
        let encrypted_password = site_credentials.get(&username.unwrap()).unwrap();
        let password = self.key_pair.decrypt(encrypted_password);

        println!("{}", password);
    }
}

fn get_sk(device_pass: &str) -> [u8;32] {
    let path_to_file = match app_dir(AppDataType::UserData, &APP_INFO, "data") {
        Ok(path) => path.join("secret_key"),
        Err(e) => panic!("Error: {}", e)
    };

    match File::open(path_to_file) {
        Ok(mut file) => {
            let mut content: [u8;32] = [0;32];
            match file.read_exact(&mut content) {
                Ok(_) => content,
                Err(e) => panic!("Error: {}", e)
            }
        },
        Err(e) => panic!("Error: {}", e)
    }
}