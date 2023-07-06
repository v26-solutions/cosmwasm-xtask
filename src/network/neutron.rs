use std::path::PathBuf;

use once_cell::unsync::OnceCell;
use xshell::{cmd, Shell};

use crate::{
    cli::{Cli, Cmd},
    key::KeyringBackend,
    Error,
};

use super::{
    gas::{Price as GasPrice, Prices as GasPrices},
    home_path_prefix, ChainId, Clean, Initialize, Instance, IntoForeground, Node, NodeUri,
    StartLocal,
};

#[derive(Default)]
pub struct Local {
    rel_src_path: PathBuf,
    rel_home_path: PathBuf,
    node_uri: OnceCell<NodeUri>,
}

pub const REPO_URL: &str = "https://github.com/neutron-org/neutron.git";
pub const REPO_BRANCH: &str = "main";
pub const REPO_CLONE_DIR: &str = "neutron_src";

pub const DOCKER_IMAGE_NAME: &str = "neutron-node";

pub const LOCAL_CHAIN_HOME_DIR: &str = ".neutrond_local";
pub const LOCAL_CHAIN_ID: &str = "test-1";
pub const LOCAL_CHAIN_DENOM: &str = "untrn";
pub const LOCAL_CONTAINER_NAME: &str = "neutron";

pub const DEMO_MNEMONIC_1: &str ="banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass";
pub const DEMO_MNEMONIC_2: &str ="veteran try aware erosion drink dance decade comic dawn museum release episode original list ability owner size tuition surface ceiling depth seminar capable only";
pub const DEMO_MNEMONIC_3: &str ="obscure canal because tomorrow tribe sibling describe satoshi kiwi upgrade bless empty math trend erosion oblige donate label birth chronic hazard ensure wreck shine";

impl Initialize for Local {
    type Instance = Instance<Local>;

    fn initialize(sh: &Shell) -> Result<Instance<Self>, Error> {
        let mut rel_src_path = sh.current_dir();
        rel_src_path.push(home_path_prefix());
        rel_src_path.push(REPO_CLONE_DIR);

        let mut rel_home_path = sh.current_dir();
        rel_home_path.push(home_path_prefix());
        rel_home_path.push(LOCAL_CHAIN_HOME_DIR);

        let mut instance = Instance::new(Local {
            rel_src_path,
            rel_home_path,
            ..Default::default()
        });

        let rel_src_path = instance.network.rel_src_path.as_path();

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

        cmd!(sh, "make build-docker-image").run()?;

        let demo_key_1 =
            instance
                .cli(sh)?
                .recover_key("local1", DEMO_MNEMONIC_1, KeyringBackend::Test)?;

        instance.keys.push(demo_key_1);

        let demo_key_2 =
            instance
                .cli(sh)?
                .recover_key("local2", DEMO_MNEMONIC_2, KeyringBackend::Test)?;

        instance.keys.push(demo_key_2);

        let demo_key_3 =
            instance
                .cli(sh)?
                .recover_key("local3", DEMO_MNEMONIC_3, KeyringBackend::Test)?;

        instance.keys.push(demo_key_3);

        Ok(instance)
    }
}

impl Cli for Instance<Local> {
    fn cli<'a>(&self, sh: &'a Shell) -> Result<Cmd<'a>, Error> {
        let src_path = self.network.rel_src_path.as_path();
        let home_path = self.network.rel_home_path.as_path();
        let cmd = cmd!(sh, "{src_path}/build/neutrond --home {home_path}");

        Ok(Cmd::from(cmd))
    }
}

pub struct LocalHandle<'a> {
    sh: &'a Shell,
}

impl<'a> IntoForeground for LocalHandle<'a> {
    fn into_foreground(self) -> Result<(), Error> {
        ctrlc::set_handler(|| {})?;

        cmd!(self.sh, "docker logs -f {LOCAL_CONTAINER_NAME}")
            .ignore_status()
            .run()?;

        Ok(())
    }
}

impl<'a> Drop for LocalHandle<'a> {
    fn drop(&mut self) {
        cmd!(self.sh, "docker stop {LOCAL_CONTAINER_NAME}")
            .ignore_status()
            .run()
            .expect("docker stop command status ignored");
    }
}

impl StartLocal for Instance<Local> {
    type Handle<'shell> = LocalHandle<'shell>;

    fn start_local<'shell>(&self, sh: &'shell Shell) -> Result<Self::Handle<'shell>, Error> {
        cmd!(
            sh,
            "docker run
                    --rm
                    --detach
                    --name {LOCAL_CONTAINER_NAME}
                    --publish 1317:1317 
                    --publish 26657:26657 
                    --publish 26656:26656 
                    --publish 16657:16657 
                    --publish 8090:9090 
                    --env RUN_BACKGROUND=0
                    {DOCKER_IMAGE_NAME}"
        )
        .run()?;

        Ok(LocalHandle { sh })
    }
}

impl Node for Instance<Local> {
    fn node_uri(&self, sh: &Shell) -> Result<NodeUri, Error> {
        self.network
            .node_uri
            .get_or_try_init(|| {
                cmd!(sh, "docker inspect")
                    .args([
                        "-f",
                        "'{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}'",
                        LOCAL_CONTAINER_NAME,
                    ])
                    .read()
                    .map(|ip| {
                        let ip = ip
                            .strip_prefix('\'')
                            .and_then(|ip| ip.strip_suffix('\''))
                            .unwrap_or(ip.as_str());
                        format!("tcp://{ip}:26657")
                    })
                    .map(NodeUri::from)
            })
            .map_err(Error::from)
            .cloned()
    }

    fn chain_id(&self) -> ChainId {
        ChainId::from(LOCAL_CHAIN_ID.to_owned())
    }
}

impl Clean for Instance<Local> {
    fn clean(&self, sh: &Shell) -> Result<(), Error> {
        let rel_src_path = self.network.rel_src_path.as_path();
        let rel_home_path = self.network.rel_home_path.as_path();

        sh.remove_path(rel_src_path)?;
        sh.remove_path(rel_home_path)?;

        Ok(())
    }
}

impl GasPrices for Instance<Local> {
    fn low_gas_price(&self) -> GasPrice {
        GasPrice::new(0.01, LOCAL_CHAIN_DENOM)
    }

    fn medium_gas_price(&self) -> GasPrice {
        GasPrice::new(0.02, LOCAL_CHAIN_DENOM)
    }

    fn high_gas_price(&self) -> GasPrice {
        GasPrice::new(0.04, LOCAL_CHAIN_DENOM)
    }
}
