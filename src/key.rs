use derive_more::Display;
use serde::Deserialize;

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
