mod api;
mod file;

use anyhow::format_err;
use std::collections::HashMap;
use std::string::String;

use app_dirs2::AppInfo;

use crate::file::{
    read_app_data, read_password_hash, read_sk, write_app_data, write_password_hash, write_sk,
};
use api::Api;
use passphrasex_common::crypto::asymmetric::{KeyPair, SeedPhrase};
use passphrasex_common::crypto::symmetric::{encrypt_data, generate_salt, hash, verify_password};
use passphrasex_common::model::password::Password;

pub const APP_INFO: AppInfo = AppInfo {
    name: "PassPhraseX",
    author: "Santos Matías Rosati",
};

// Map of site -> Map of username -> password
pub type CredentialsMap = HashMap<String, HashMap<String, Password>>;

pub struct App {
    key_pair: KeyPair,
    credentials: CredentialsMap,
    api: Api,
}

pub async fn register(device_pass: &str) -> anyhow::Result<SeedPhrase> {
    let salt = generate_salt()?;
    let pass_hash = hash(device_pass, &salt)?;

    let seed_phrase = SeedPhrase::new();
    let key_pair = KeyPair::new(seed_phrase.clone());

    let api = Api::new(key_pair.clone());

    write_password_hash(&pass_hash)?;

    let enc = encrypt_data(&pass_hash.cipher, key_pair.private_key.as_bytes())?;

    let mut sk_bytes: [u8; 32] = [0; 32];
    sk_bytes.copy_from_slice(enc.as_slice());
    write_sk(key_pair.private_key.as_bytes(), &pass_hash.cipher)?;

    write_app_data(&HashMap::new())?;

    api.create_user(key_pair.get_pk()).await?;

    Ok(seed_phrase)
}

pub async fn auth_device(seed_phrase: &str, device_pass: &str) -> anyhow::Result<()> {
    let salt = generate_salt()?;
    let pass_hash = hash(device_pass, &salt)?;

    let seed_phrase = SeedPhrase::from(seed_phrase.to_string());
    let key_pair = KeyPair::new(seed_phrase.clone());

    let api = Api::new(key_pair.clone());

    write_password_hash(&pass_hash)?;

    write_sk(key_pair.private_key.as_bytes(), &pass_hash.cipher)?;

    sync_with_api(api, key_pair.clone()).await?;

    Ok(())
}

async fn sync_with_api(api: Api, key_pair: KeyPair) -> anyhow::Result<CredentialsMap> {
    let passwords = api.get_passwords(key_pair.get_pk()).await?;
    let mut credentials: CredentialsMap = HashMap::new();

    for password in passwords {
        credentials
            .entry(password.site.clone())
            .or_insert(HashMap::new())
            .insert(password._id.clone(), password.clone());
    }

    write_app_data(&credentials)?;

    Ok(credentials)
}

impl App {
    pub async fn new(device_pass: &str) -> anyhow::Result<App> {
        let pass_hash = read_password_hash()?;
        verify_password(device_pass, &pass_hash.cipher, &pass_hash.nonce)?;

        let private_key = read_sk(&pass_hash.cipher)?;
        let key_pair = KeyPair::from_sk(private_key);

        let api = Api::new(key_pair.clone());

        let credentials = sync_with_api(api, key_pair.clone()).await.or_else(|_| {
            println!("Failed to sync with API, using local data");
            read_app_data()
        })?;

        Ok(App {
            key_pair: key_pair.clone(),
            credentials,
            api: Api::new(key_pair),
        })
    }

    pub async fn add(
        &mut self,
        site: String,
        username: String,
        password: String,
    ) -> anyhow::Result<()> {
        self.verify_credentials_dont_exist(&site, &username)?;

        let user_id = self.key_pair.get_pk();

        let password_id = self.key_pair.hash(&format!("{}{}", site, username))?;

        let password = Password {
            _id: password_id.clone(),
            user_id: user_id.clone(),
            site: site.clone(),
            username,
            password,
        };
        let password = password.encrypt(&self.key_pair);

        self.api.add_password(user_id, password.clone()).await?;

        self.credentials
            .entry(site)
            .or_insert(HashMap::new())
            .insert(password_id, password);

        write_app_data(&self.credentials).expect("Failed to save app data to file");
        Ok(())
    }

    pub async fn get(
        &mut self,
        site: String,
        username: Option<String>,
    ) -> anyhow::Result<Vec<Password>> {
        match self.credentials.get(&site) {
            Some(passwords) => match username {
                Some(username) => {
                    let id = self.key_pair.hash(&format!("{}{}", site, username))?;
                    let password = passwords
                        .get(&id)
                        .ok_or(format_err!("Password not found"))?;

                    Ok(vec![password.decrypt(&self.key_pair)])
                }
                None => {
                    let result = passwords
                        .iter()
                        .map(|(_, password)| password.decrypt(&self.key_pair))
                        .collect();

                    Ok(result)
                }
            },
            None => Err(format_err!("No passwords found")),
        }
    }

    pub async fn edit(
        &mut self,
        site: String,
        username: String,
        password: String,
    ) -> anyhow::Result<()> {
        self.verify_credentials_exist(&site, &username)?;

        let user_id = self.key_pair.get_pk();
        let password_id = self.key_pair.hash(&format!("{}{}", site, username))?;

        let password_enc = self.key_pair.encrypt(&password);
        self.api
            .edit_password(user_id, password_id.clone(), password_enc.clone().into())
            .await?;

        self.credentials
            .entry(site)
            .or_insert(HashMap::new()) // Should never happen
            .entry(password_id)
            .and_modify(|e| e.password = password_enc.clone().into());

        write_app_data(&self.credentials).expect("Failed to save app data to file");

        Ok(())
    }

    pub async fn delete(&mut self, site: String, username: String) -> anyhow::Result<()> {
        self.verify_credentials_exist(&site, &username)?;

        let user_id = self.key_pair.get_pk();
        let password_id = self.key_pair.hash(&format!("{}{}", site, username))?;

        self.api
            .delete_password(user_id, password_id.clone())
            .await?;

        self.credentials
            .entry(site)
            .or_insert(HashMap::new()) // Should never happen
            .remove(&password_id);

        write_app_data(&self.credentials).expect("Failed to save app data to file");

        Ok(())
    }

    fn verify_credentials_exist(&self, site: &str, username: &str) -> anyhow::Result<()> {
        match self.credentials.get(site) {
            Some(passwords) => {
                let id = self.key_pair.hash(&format!("{}{}", site, username))?;
                passwords
                    .get(&id)
                    .ok_or(format_err!("Credentials not found"))?;
                Ok(())
            }
            None => Err(format_err!("Credentials not found")),
        }
    }

    fn verify_credentials_dont_exist(&self, site: &str, username: &str) -> anyhow::Result<()> {
        match self.verify_credentials_exist(site, username) {
            Ok(_) => Err(format_err!("Credentials already exist")),
            Err(_) => Ok(()),
        }
    }
}
