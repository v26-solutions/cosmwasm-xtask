use derive_more::{Display, From, FromStr};
use xshell::Shell;

use crate::{
    cli::Cli,
    key::{Key, KeyringBackend},
    Error,
};

pub mod archway;

pub mod neutron {
    pub mod local;
    pub mod testnet;
}

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

pub trait Keys: Cli {
    fn keys(&self) -> &[Key];

    /// Recover a key with the given `mnemonic` & add it to the network's keys as `name` in the given `backend`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the key recovery or additions commands fail.
    fn recover(
        &mut self,
        sh: &Shell,
        name: &str,
        mnemonic: &str,
        backend: KeyringBackend,
    ) -> Result<Key, Error>;
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
    type Handle<'shell>: IntoForeground;

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
    /// This function will return an error depending on the implementation.
    fn clean_state(sh: &Shell) -> Result<(), Error>;

    /// Remove all artifacts
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn clean_all(sh: &Shell) -> Result<(), Error>;
}

pub struct Instance<Network> {
    pub keys: Vec<Key>,
    pub network: Network,
}

macro_rules! home_path_prefix {
    () => {{
        let mut path = String::new();
        path.push_str("target/");
        path.push_str(&module_path!());
        let path = path.replace("::", "/");
        std::path::PathBuf::from(path)
    }};
}

macro_rules! concat_paths {
    ($root:expr, $($rel_path:expr),+) => {{
        let mut p = $root;
        $(p.push($rel_path);)+
        p
    }};
}

macro_rules! make_abs_root {
    ($sh:ident) => {{
        $crate::network::concat_paths!($sh.current_dir(), $crate::network::home_path_prefix!())
    }};
}

macro_rules! make_abs_path {
    ($sh:ident, $($rel_path:expr),+) => {{
        $crate::network::concat_paths!($crate::network::make_abs_root!($sh), $($rel_path),+)
    }};
}

pub(crate) use concat_paths;
pub(crate) use home_path_prefix;
pub(crate) use make_abs_path;
pub(crate) use make_abs_root;

impl<Network> Instance<Network> {
    pub fn new(network: Network) -> Self {
        Self {
            keys: vec![],
            network,
        }
    }

    fn network(&self) -> &Network {
        &self.network
    }
}

impl<Network> Keys for Instance<Network>
where
    Self: Cli,
{
    fn keys(&self) -> &[Key] {
        &self.keys
    }

    fn recover(
        &mut self,
        sh: &Shell,
        name: &str,
        mnemonic: &str,
        backend: KeyringBackend,
    ) -> Result<Key, Error> {
        let key = self.cli(sh)?.recover_key(name, mnemonic, backend)?;

        self.keys.push(key.clone());

        Ok(key)
    }
}
