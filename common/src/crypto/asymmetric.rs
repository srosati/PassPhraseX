use crate::crypto::common::EncryptedValue;
use crate::crypto::symmetric::hash;
use base64::{engine::general_purpose::URL_SAFE, Engine};
use bip32::{Mnemonic, XPrv};
use crypto_box::aead::{Aead, AeadCore, OsRng, Payload};
use crypto_box::{ChaChaBox, Nonce, PublicKey, SecretKey};
use std::str;

#[derive(Clone)]
pub struct SeedPhrase {
    phrase: String,
}

impl SeedPhrase {
    pub fn new() -> SeedPhrase {
        let mnemonic = Mnemonic::random(OsRng, Default::default());
        SeedPhrase {
            phrase: mnemonic.phrase().to_owned(),
        }
    }

    pub fn get_phrase(&self) -> String {
        self.phrase.clone()
    }
}

impl Default for SeedPhrase {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for SeedPhrase {
    fn from(value: String) -> SeedPhrase {
        SeedPhrase { phrase: value }
    }
}

#[derive(Clone)]
pub struct KeyPair {
    pub private_key: SecretKey,
    pub public_key: PublicKey,
}

/*
* Implement asymmetric encryption functions for struct KeyPair
* 2 methods -> encrypt & decrypt
*/
impl KeyPair {
    pub fn new(seed_phrase: SeedPhrase) -> KeyPair {
        Self::try_new(seed_phrase).expect("Failed to create key pair")
    }

    pub fn try_new(seed_phrase: SeedPhrase) -> anyhow::Result<KeyPair> {
        // Get Mnemonic using the default language (English)
        let mnemonic = Mnemonic::new(seed_phrase.get_phrase(), Default::default())
            .map_err(|_| anyhow::format_err!("Failed to create mnemonic"))?;

        // Derive a BIP39 seed value using the given password
        let seed = mnemonic.to_seed("");

        // Derive the root `XPrv` from the `seed` value
        let derived_sk =
            XPrv::new(&seed).map_err(|_| anyhow::format_err!("Failed to derive sk"))?;

        // Convert the `XPrv` to a `SecretKey` and `PublicKey`
        let private_key = SecretKey::from(derived_sk.to_bytes());
        let public_key = private_key.public_key();

        Ok(KeyPair {
            private_key,
            public_key,
        })
    }

    pub fn from_sk(sk: [u8; 32]) -> KeyPair {
        let private_key = SecretKey::from(sk);
        let public_key = private_key.public_key();
        KeyPair {
            private_key,
            public_key,
        }
    }

    pub fn encrypt(&self, message: &str) -> EncryptedValue {
        let nonce = ChaChaBox::generate_nonce(&mut OsRng);

        let personal_box = ChaChaBox::new(&self.public_key, &self.private_key);
        let enc = personal_box
            .encrypt(
                &nonce,
                Payload {
                    msg: message.as_bytes(),
                    aad: b"",
                },
            )
            .unwrap();
        EncryptedValue {
            cipher: URL_SAFE.encode(enc),
            nonce: URL_SAFE.encode(nonce),
        }
    }

    pub fn decrypt(&self, enc: &EncryptedValue) -> String {
        let cipher = URL_SAFE.decode(enc.cipher.as_bytes()).unwrap();
        let personal_box = ChaChaBox::new(&self.public_key, &self.private_key);

        let nonce = URL_SAFE
            .decode(enc.nonce.as_bytes())
            .expect("Failed to decode nonce");
        let mut content: [u8; 24] = [0; 24];
        content.copy_from_slice(&nonce);
        let dec = personal_box
            .decrypt(
                &Nonce::from(content),
                Payload {
                    msg: cipher.as_slice(),
                    aad: b"",
                },
            )
            .unwrap();

        str::from_utf8(&dec).unwrap().to_owned()
    }

    pub fn sign(&self, message: &str) -> EncryptedValue {
        let nonce = ChaChaBox::generate_nonce(&mut OsRng);
        let verifiable_box =
            ChaChaBox::new(&SecretKey::from([0; 32]).public_key(), &self.private_key);
        let enc = verifiable_box
            .encrypt(
                &nonce,
                Payload {
                    msg: message.as_bytes(),
                    aad: b"",
                },
            )
            .unwrap();
        EncryptedValue {
            cipher: URL_SAFE.encode(enc),
            nonce: URL_SAFE.encode(nonce),
        }
    }

    pub fn hash(&self, message: &str) -> anyhow::Result<String> {
        Ok(hash(message, &URL_SAFE.encode(&self.public_key))
            .map_err(|_| anyhow::format_err!("Failed to hash"))?
            .cipher)
    }

    pub fn get_pk(&self) -> String {
        URL_SAFE.encode(&self.public_key)
    }
}

pub fn public_key_from_base64(pk: &str) -> PublicKey {
    let pk_bytes = URL_SAFE.decode(pk.as_bytes()).unwrap();
    let mut buff: [u8; 32] = [0; 32];
    buff.copy_from_slice(pk_bytes.as_slice());
    PublicKey::from(buff)
}

pub fn verify(public_key: &PublicKey, value: EncryptedValue) -> anyhow::Result<String> {
    let verifiable_box = ChaChaBox::new(public_key, &SecretKey::from([0; 32]));

    let nonce = URL_SAFE.decode(value.nonce.as_bytes())?;
    let mut content: [u8; 24] = [0; 24];
    content.copy_from_slice(&nonce);

    let cipher = URL_SAFE.decode(value.cipher.as_bytes())?;
    let payload = Payload {
        msg: cipher.as_slice(),
        aad: b"",
    };

    match verifiable_box.decrypt(&Nonce::from(content), payload) {
        Ok(dec) => Ok(str::from_utf8(&dec)?.to_owned()),
        Err(_) => Err(anyhow::format_err!("Failed to decrypt")),
    }
}
