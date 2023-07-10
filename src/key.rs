use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use bip39::Mnemonic;
use derive_more::Display;
use nanorand::{Rng, WyRand};
use serde::Deserialize;

use crate::Error;

#[derive(Debug, Display, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum KeyringBackend {
    Os,
    Test,
}

impl KeyringBackend {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            KeyringBackend::Os => "os",
            KeyringBackend::Test => "test",
        }
    }
}

#[derive(Debug, Display, Deserialize, Clone, PartialEq, Eq)]
#[display(fmt = "{name} {address}")]
pub struct Raw {
    name: String,
    address: String,
}

impl Raw {
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[must_use]
    pub fn address(&self) -> &str {
        self.address.as_str()
    }

    #[must_use]
    pub fn with_backend(self, backend: KeyringBackend) -> Key {
        Key { raw: self, backend }
    }
}

#[derive(Debug, Clone, Display, Deserialize, PartialEq, Eq)]
#[display(fmt = "{raw} ({backend})")]
pub struct Key {
    raw: Raw,
    backend: KeyringBackend,
}

impl Key {
    #[must_use]
    pub fn name(&self) -> &str {
        self.raw.name()
    }

    #[must_use]
    pub fn address(&self) -> &str {
        self.raw.address()
    }

    #[must_use]
    pub fn backend(&self) -> &str {
        self.backend.as_str()
    }
}

/// Generate a BIP-39 Mnemonic string using entropy from the operating system
/// to seed the RNG.
///
/// WARNING: Do not use for real wallets.
///
/// # Errors
///
/// This function will return an error if:
pub fn generate_mnemonic() -> Result<String, Error> {
    let mut rng = WyRand::new();

    let mut bytes = [0u8; 16];

    rng.fill_bytes(&mut bytes);

    let mnemomic = Mnemonic::from_entropy(&bytes)?;

    Ok(mnemomic.to_string())
}

/// Generate a BIP-39 Mnemonic string using the provided `seed` for the RNG
///
/// WARNING: Do not use for real wallets.
///
/// # Errors
///
/// This function will return an error if:
pub fn generate_mnemonic_with_seed(seed: &str) -> Result<String, Error> {
    let mut hasher = DefaultHasher::default();

    seed.hash(&mut hasher);

    let seed = hasher.finish();

    let mut rng = WyRand::new_seed(seed);

    let mut bytes = [0u8; 16];

    rng.fill_bytes(&mut bytes);

    let mnemomic = Mnemonic::from_entropy(&bytes)?;

    Ok(mnemomic.to_string())
}
