use std::path::PathBuf;

use derive_more::{Display, From, FromStr};
use xshell::Shell;

use crate::{cli::Cli, key::Key, Error};

pub mod archway;
pub mod neutron;

#[derive(Debug, Display, From, Clone)]
pub struct NodeUri(String);

impl NodeUri {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Display, From, FromStr, Clone)]
pub struct ChainId(String);

impl ChainId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

pub mod gas {
    use derive_more::{Display, From};

    #[derive(Debug, Display, From, Clone)]
    pub enum Amount {
        Int(u128),
        Decimal(f64),
    }

    #[derive(Debug, Display, Clone)]
    #[display(fmt = "{amount}{denom}")]
    pub struct Price {
        amount: Amount,
        denom: String,
    }

    #[derive(Debug, Display, From, Clone)]
    pub struct Units(u128);

    impl Price {
        pub fn new(amount: impl Into<Amount>, denom: impl Into<String>) -> Self {
            Self {
                amount: amount.into(),
                denom: denom.into(),
            }
        }

        pub fn units(self, units: impl Into<Units>) -> Gas {
            Gas {
                units: units.into(),
                price: self,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Gas {
        pub units: Units,
        pub price: Price,
    }

    pub trait Prices {
        fn low_gas_price(&self) -> Price;

        fn medium_gas_price(&self) -> Price;

        fn high_gas_price(&self) -> Price;
    }
}

pub trait Node {
    /// Obtain the URI for the node
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn node_uri(&self, sh: &Shell) -> Result<NodeUri, Error>;

    fn chain_id(&self) -> ChainId;
}

pub trait Keys {
    fn keys(&self) -> &[Key];
}

pub trait Network: Node + Cli + Keys + gas::Prices {}

impl<T> Network for T where T: Node + Cli + Keys + gas::Prices {}

pub trait Initialize {
    type Instance: Network;

    /// Inititise network resources and/or any required state
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn initialize(sh: &Shell) -> Result<Self::Instance, Error>;
}

pub trait IntoForeground {
    /// Consume a `StartLocal::Handle` to bring it to the foreground & follow it's logs until Ctrl + C is received
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn into_foreground(self) -> Result<(), Error>;
}

pub trait StartLocal {
    type Handle<'shell>: IntoForeground + Drop;

    /// Start a local node in the background, returning a handle which acts as a RAII guard to stop the node when dropped
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn start_local<'shell>(&self, sh: &'shell Shell) -> Result<Self::Handle<'shell>, Error>;
}

pub trait Clean {
    /// Remove any network state
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn clean(&self, sh: &Shell) -> Result<(), Error>;
}

pub struct Instance<Network> {
    pub keys: Vec<Key>,
    pub network: Network,
}

pub const DEFAULT_HOME_DIR: &str = "target/cosmwasm_xtask";

pub fn home_path_prefix() -> PathBuf {
    std::env::var("COSMWASM_XTASK_HOME_DIR")
        .map_or_else(|_| PathBuf::from(DEFAULT_HOME_DIR), PathBuf::from)
}

impl<Network> Instance<Network> {
    pub fn new(network: Network) -> Self {
        Self {
            keys: vec![],
            network,
        }
    }
}

impl<Network> Keys for Instance<Network> {
    fn keys(&self) -> &[Key] {
        &self.keys
    }
}
