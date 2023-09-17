use std::path::PathBuf;

use xshell::{cmd, Shell};

use crate::{
    cli::{Cli, Cmd},
    key::KeyringBackend,
    network::{
        gas::{Price as GasPrice, Prices as GasPrices},
        make_abs_path, make_abs_root, ChainId, Clean, Initialize, Instance, Node, NodeUri,
    },
    Error,
};

pub const REPO_URL: &str = "https://github.com/neutron-org/neutron.git";
pub const REPO_BRANCH: &str = "main";
pub const REPO_CLONE_DIR: &str = "src";
pub const NODE: &str = "https://rpc-t.neutron.nodestake.top:443";
pub const CHAIN_HOME_DIR: &str = "data";
pub const CHAIN_ID: &str = "pion-1";
pub const CHAIN_DENOM: &str = "untrn";

#[derive(Default)]
pub struct Testnet {
    src_path: PathBuf,
    home_path: PathBuf,
}

impl Initialize for Testnet {
    type Instance = Instance<Testnet>;

    fn initialize(sh: &Shell) -> Result<Instance<Self>, Error> {
        let mut instance = Instance::new(Testnet {
            src_path: make_abs_path!(sh, REPO_CLONE_DIR),
            home_path: make_abs_path!(sh, CHAIN_HOME_DIR),
        });

        let rel_src_path = instance.network.src_path.as_path();

        if sh.path_exists(rel_src_path) {
            let keys = instance.cli(sh)?.list_keys(KeyringBackend::Test)?;
            instance.keys = keys;
            return Ok(instance);
        }

        cmd!(
            sh,
            "git clone --depth 1 --branch {REPO_BRANCH} {REPO_URL} {rel_src_path}"
        )
        .run()?;

        let _cd = sh.push_dir(rel_src_path);

        cmd!(sh, "make build").run()?;

        Ok(instance)
    }
}

impl Cli for Instance<Testnet> {
    fn cli<'a>(&self, sh: &'a Shell) -> Result<Cmd<'a>, Error> {
        let src_path = self.network.src_path.as_path();
        let home_path = self.network.home_path.as_path();
        let cmd = cmd!(sh, "{src_path}/build/neutrond --home {home_path}");

        Ok(Cmd::from(cmd))
    }
}

impl Node for Instance<Testnet> {
    fn node_uri(&self, _sh: &Shell) -> Result<NodeUri, Error> {
        Ok(NodeUri::from(NODE.to_owned()))
    }

    fn chain_id(&self) -> ChainId {
        ChainId::from(CHAIN_ID.to_owned())
    }
}

impl Clean for Testnet {
    fn clean_state(sh: &Shell) -> Result<(), Error> {
        sh.remove_path(make_abs_path!(sh, CHAIN_HOME_DIR)).ok();
        Ok(())
    }

    fn clean_all(sh: &Shell) -> Result<(), Error> {
        sh.remove_path(make_abs_root!(sh)).ok();
        Ok(())
    }
}

impl GasPrices for Instance<Testnet> {
    fn low_gas_price(&self) -> GasPrice {
        GasPrice::new(0.001, CHAIN_DENOM)
    }

    fn medium_gas_price(&self) -> GasPrice {
        GasPrice::new(0.002, CHAIN_DENOM)
    }

    fn high_gas_price(&self) -> GasPrice {
        GasPrice::new(0.004, CHAIN_DENOM)
    }
}
