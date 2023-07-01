use std::path::{Path, PathBuf};

use derive_more::{Display, From, FromStr};
use xshell::Shell;

use crate::{key::Key, Error};

pub mod archway;

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

    #[derive(Debug, Display, Clone)]
    #[display(fmt = "{amount}{denom}")]
    pub struct Price {
        amount: u128,
        denom: String,
    }

    #[derive(Debug, Display, From, Clone)]
    pub struct Units(u128);

    impl Price {
        pub fn new(amount: u128, denom: impl Into<String>) -> Self {
            Self {
                amount,
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

pub trait Initialize {
    type Instance;

    /// Inititise network resources and/or any required state
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn initialize(sh: &Shell) -> Result<Self::Instance, Error>;
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

pub trait StartLocal {
    /// Start a local node
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn start_local(&self, sh: &Shell) -> Result<(), Error>;
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
    pub rel_home_path: PathBuf,
    pub network: Network,
}

pub const DEFAULT_HOME_DIR: &str = "target/cosmwasm_xtask";

impl<Network> Instance<Network> {
    pub fn new<P>(home_dir: P, network: Network) -> Self
    where
        P: AsRef<Path>,
    {
        let mut rel_home_path = std::env::var("COSMWASM_XTASK_HOME_DIR")
            .map_or_else(|_| PathBuf::from(DEFAULT_HOME_DIR), PathBuf::from);

        rel_home_path.push(home_dir);

        Self {
            keys: vec![],
            rel_home_path,
            network,
        }
    }

    pub fn abs_home_path(&self, sh: &Shell) -> PathBuf {
        let mut abs_home_path = sh.current_dir();
        abs_home_path.push(&self.rel_home_path);
        abs_home_path
    }
}
