use std::path::PathBuf;

use once_cell::unsync::OnceCell;
use xshell::{cmd, Shell};

use crate::{
    cli::{Cli, Cmd, ReadyTxCmd},
    contract::{Execute, Tx},
    key::KeyringBackend,
    Error,
};

use super::{
    gas::{Price as GasPrice, Prices as GasPrices},
    make_abs_path, make_abs_root, ChainId, Clean, Initialize, Instance, IntoForeground, Node,
    NodeUri, StartLocal,
};

pub trait CmdExt: Sized {
    #[must_use]
    fn fees(self, amount: u128, denom: &str) -> Self;
}

impl<'a> CmdExt for ReadyTxCmd<'a> {
    fn fees(mut self, amount: u128, denom: &str) -> Self {
        self.cmd = self
            .cmd
            .args(["--fees", format!("{amount}{denom}").as_str()]);
        self
    }
}

impl<Msg, Response> CmdExt for Tx<Execute, Msg, Response> {
    fn fees(self, amount: u128, denom: &str) -> Self {
        let denom = denom.to_owned();
        self.pre_execute_hook(move |cmd| cmd.fees(amount, denom.as_str()))
    }
}

#[derive(Default)]
pub struct Local {
    home_path: PathBuf,
    node_uri: OnceCell<NodeUri>,
}

pub const LOCAL_HOME_DIR: &str = "data";
pub const LOCAL_CHAIN_ID: &str = "localnet";
pub const LOCAL_CHAIN_MONIKER: &str = "archway-local";
pub const LOCAL_CHAIN_DENOM: &str = "stake";
pub const LOCAL_CONTAINER_NAME: &str = "cosmwasm_xtask_archwayd";

impl Initialize for Local {
    type Instance = Instance<Local>;

    fn initialize(sh: &Shell) -> Result<Self::Instance, Error> {
        cmd!(sh, "docker pull ghcr.io/archway-network/archwayd:v1.0.0")
            .ignore_stdout()
            .ignore_stderr()
            .quiet()
            .run()?;

        let mut instance = Instance::new(Local {
            home_path: make_abs_path!(sh, LOCAL_HOME_DIR),
            ..Default::default()
        });

        if sh.path_exists(&instance.network.home_path) {
            let keys = instance.cli(sh)?.list_keys(KeyringBackend::Test)?;
            instance.keys = keys;
            return Ok(instance);
        }

        sh.create_dir(&instance.network.home_path)?;

        let chain_id = instance.chain_id();

        instance
            .cli(sh)?
            .init_chain(LOCAL_CHAIN_MONIKER, &chain_id)?;

        let local0 = instance.cli(sh)?.add_key("local0", KeyringBackend::Test)?;

        instance.cli(sh)?.add_genesis_account(
            &local0,
            &[(1_000_000_000_000_000_000_000_000, LOCAL_CHAIN_DENOM)],
        )?;

        let local1 = instance.cli(sh)?.add_key("local1", KeyringBackend::Test)?;

        instance.cli(sh)?.add_genesis_account(
            &local1,
            &[(1_000_000_000_000_000_000_000_000, LOCAL_CHAIN_DENOM)],
        )?;

        instance.cli(sh)?.gentx(
            &local0,
            9_500_000_000_000_000_000,
            LOCAL_CHAIN_DENOM,
            LOCAL_CHAIN_ID,
        )?;

        instance.keys.push(local0);

        instance.keys.push(local1);

        instance.cli(sh)?.collect_gentx()?;

        instance.cli(sh)?.validate_genesis()?;

        cmd!(
            sh,
            "docker pull ghcr.io/archway-network/archwayd-debug:v1.0.0"
        )
        .ignore_stdout()
        .ignore_stderr()
        .run()?;

        let abs_home_path = instance.network.home_path.as_path();

        cmd!(
            sh,
            "docker run 
                    --rm 
                    --interactive 
                    --volume {abs_home_path}:/home 
                    --entrypoint /bin/sed
                    ghcr.io/archway-network/archwayd-debug:v1.0.0
                    -i 's/127.0.0.1/0.0.0.0/g' /home/config/config.toml"
        )
        .run()?;

        cmd!(
            sh,
            "docker run 
                    --rm 
                    --interactive 
                    --volume {abs_home_path}:/home 
                    --entrypoint /bin/sed
                    ghcr.io/archway-network/archwayd-debug:v1.0.0"
        )
        .args([
            "-i",
            r#"s/cors_allowed_origins = \[\]/cors_allowed_origins = \["*"\]/g"#,
            "/home/config/config.toml",
        ])
        .run()?;

        Ok(instance)
    }
}

impl Cli for Instance<Local> {
    fn cli<'a>(&self, sh: &'a Shell) -> Result<Cmd<'a>, Error> {
        let current_dir = sh.current_dir();

        let abs_home_path = self.network.home_path.as_path();

        let cmd = cmd!(
            sh,
            "docker run 
                    --rm 
                    --interactive 
                    --volume {abs_home_path}:/home 
                    --volume {current_dir}:/work 
                    --workdir /work 
                    ghcr.io/archway-network/archwayd:v1.0.0
                    --home /home
                    "
        );

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
        let cwd = sh.current_dir();

        let abs_home_path = self.network.home_path.as_path();

        cmd!(
            sh,
            "docker run
                    --rm
                    --detach
                    --name {LOCAL_CONTAINER_NAME}
                    --volume {abs_home_path}:/home 
                    --volume {cwd}:/work 
                    --workdir /work 
                    --publish 9090:9090
                    --publish 26657:26657
                    ghcr.io/archway-network/archwayd:v1.0.0
                    start
                    --home /home"
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

impl Clean for Local {
    fn clean_state(sh: &Shell) -> Result<(), Error> {
        let cwd = make_abs_root!(sh);

        let home_path = make_abs_path!(sh, LOCAL_HOME_DIR);

        cmd!(
            sh,
            "docker run 
                    --rm 
                    --interactive 
                    --volume {cwd}:/work 
                    --workdir /work 
                    --entrypoint /bin/rm
                    ghcr.io/archway-network/archwayd-debug:v1.0.0
                    -rf {home_path}"
        )
        .run()?;

        Ok(())
    }

    fn clean_all(sh: &Shell) -> Result<(), Error> {
        Self::clean_state(sh)?;

        cmd!(
            sh,
            "docker rmi ghcr.io/archway-network/archwayd-debug:v1.0.0"
        )
        .run()?;

        Ok(())
    }
}

impl GasPrices for Instance<Local> {
    fn low_gas_price(&self) -> GasPrice {
        GasPrice::new(10, LOCAL_CHAIN_DENOM)
    }

    fn medium_gas_price(&self) -> GasPrice {
        GasPrice::new(100, LOCAL_CHAIN_DENOM)
    }

    fn high_gas_price(&self) -> GasPrice {
        GasPrice::new(1000, LOCAL_CHAIN_DENOM)
    }
}
