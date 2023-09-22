use std::{
    fs::File,
    io::{prelude::*, BufReader},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use duct::{Expression as DuctExpression, Handle as DuctHandle};
use log::{error, info};
use xshell::{cmd, Cmd as ShellCmd, Shell};

use crate::{
    cli::{wait_for_blocks_fn, Cli, Cmd},
    key::{Key, KeyringBackend},
    network::{
        concat_paths,
        gas::{Price as GasPrice, Prices as GasPrices},
        home_path_prefix, make_abs_path, make_abs_root, ChainId, Clean, Initialize, Instance,
        IntoForeground, Node, NodeUri, StartLocal,
    },
    Error,
};

pub const NTRN_REPO_URL: &str = "https://github.com/neutron-org/neutron.git";
pub const NTRN_REPO_BRANCH: &str = "main";
pub const NTRN_REPO_CLONE_DIR: &str = "neutron/src";
pub const NTRN_BIN_PATH: &str = "bin/neutrond";
pub const NTRN_LOGFILE: &str = "neutron/neutrond.log";
pub const NTRN_CHAIN_HOME_DIR: &str = "neutron/data";
pub const NTRN_CHAIN_ID: &str = "test-1";
pub const NTRN_CHAIN_DENOM: &str = "untrn";
pub const NTRN_P2P_PORT: u16 = 26656;
pub const NTRN_RPC_PORT: u16 = 26657;
pub const NTRN_REST_PORT: u16 = 1317;
pub const NTRN_GRPC_PORT: u16 = 8090;
pub const NTRN_GRPC_WEB_PORT: u16 = 8091;
pub const NTRN_ROSETTA_PORT: u16 = 8080;

pub const GAIA_REPO_URL: &str = "https://github.com/cosmos/gaia.git";
pub const GAIA_REPO_BRANCH: &str = "v9.0.3";
pub const GAIA_REPO_CLONE_DIR: &str = "gaia/src";
pub const GAIA_BIN_PATH: &str = "bin/gaiad";
pub const GAIA_LOGFILE: &str = "gaia/gaiad.log";
pub const GAIA_CHAIN_HOME_DIR: &str = "gaia/data";
pub const GAIA_CHAIN_ID: &str = "test-2";
pub const GAIA_CHAIN_DENOM: &str = "uatom";
pub const GAIA_P2P_PORT: u16 = 16656;
pub const GAIA_RPC_PORT: u16 = 16657;
pub const GAIA_REST_PORT: u16 = 1316;
pub const GAIA_GRPC_PORT: u16 = 9090;
pub const GAIA_GRPC_WEB_PORT: u16 = 9091;
pub const GAIA_ROSETTA_PORT: u16 = 8081;

pub const HERMES_CRATE: &str = "ibc-relayer-cli";
pub const HERMES_CRATE_VERSION: &str = "1.6.0";
pub const HERMES_CRATE_BIN: &str = "hermes";
pub const HERMES_BIN_PATH: &str = "bin/hermes";
pub const HERMES_HOME_DIR: &str = ".hermes";
pub const HERMES_LOGFILE: &str = ".hermes/hermes.log";
pub const HERMES_CONFIG_FILE: &str = "config.toml";
pub const HERMES_COPY_CONFIG_PATH: &str = "network/hermes/config.toml";

pub const ICQ_RLY_REPO_URL: &str = "https://github.com/neutron-org/neutron-query-relayer.git";
pub const ICQ_RLY_REPO_BRANCH: &str = "main";
pub const ICQ_RLY_REPO_CLONE_DIR: &str = "icq_rly/src";
pub const ICQ_RLY_DB_PATH: &str = "icq_rly/db";
pub const ICQ_RLY_BIN_PATH: &str = "bin/neutron_query_relayer";
pub const ICQ_RLY_LOGFILE: &str = "icq_rly/icq_rly.log";

pub const IBC_ATOM_DENOM: &str = "uibcatom";
pub const IBC_USDC_DENOM: &str = "uibcusdc";

pub const GENESIS_ALLOCATION: u128 = 100_000_000_000_000;

pub const DEMO_MNEMONIC_1: &str = "banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass";
pub const DEMO_MNEMONIC_2: &str = "veteran try aware erosion drink dance decade comic dawn museum release episode original list ability owner size tuition surface ceiling depth seminar capable only";
pub const DEMO_MNEMONIC_3: &str = "obscure canal because tomorrow tribe sibling describe satoshi kiwi upgrade bless empty math trend erosion oblige donate label birth chronic hazard ensure wreck shine";
pub const VAL_MNEMONIC_1: &str = "clock post desk civil pottery foster expand merit dash seminar song memory figure uniform spice circle try happy obvious trash crime hybrid hood cushion";
pub const VAL_MNEMONIC_2: &str = "angry twist harsh drastic left brass behave host shove marriage fall update business leg direct reward object ugly security warm tuna model broccoli choice";
pub const RLY_MNEMONIC_1: &str = "alley afraid soup fall idea toss can goose become valve initial strong forward bright dish figure check leopard decide warfare hub unusual join cart";
pub const RLY_MNEMONIC_2: &str = "record gift you once hip style during joke field prize dust unique length more pencil transfer quit train device arrive energy sort steak upset";

macro_rules! find_and_replace_in_file {
    ($sh:expr, $file_path:expr, $($pattern:expr => $replace:expr),+) => {
        let path = concat_paths!($sh.current_dir(), $file_path);
        let mut file = $sh.read_file(&path)?;
        $(
            let replacement = format!($replace);
            let indices: Vec<_> = file.rmatch_indices($pattern)
                .map(|(start, substring)| (start, start + substring.len()))
                .collect();

            for (start, end) in indices {
                file.replace_range(start..end, &replacement);
            }
        )+
        $sh.write_file(path, file)?;
    };
}

struct InitParams<'a> {
    chain_id: &'a str,
    stake_denom: &'a str,
    p2p_port: u16,
    rpc_port: u16,
    rest_port: u16,
    rosetta_port: u16,
}

fn init_chain<'a, CliFn>(
    sh: &'a Shell,
    cli: CliFn,
    home_dir: &Path,
    InitParams {
        chain_id,
        stake_denom,
        p2p_port,
        rpc_port,
        rest_port,
        rosetta_port,
    }: InitParams,
) -> Result<Vec<Key>, Error>
where
    CliFn: Fn() -> Cmd<'a>,
{
    let pairs = [
        ("local1", DEMO_MNEMONIC_1),
        ("local2", DEMO_MNEMONIC_2),
        ("local3", DEMO_MNEMONIC_3),
        ("val1", VAL_MNEMONIC_1),
        ("val2", VAL_MNEMONIC_2),
        ("rly1", RLY_MNEMONIC_1),
        ("rly2", RLY_MNEMONIC_2),
    ];

    let mut keys = vec![];

    cli().init_chain("test", &ChainId::from(chain_id.to_owned()))?;

    for (key, mnem) in pairs {
        let key = cli().recover_key(key, mnem, KeyringBackend::Test)?;

        cli().add_genesis_account(
            &key,
            &[
                (GENESIS_ALLOCATION, stake_denom),
                (GENESIS_ALLOCATION, IBC_ATOM_DENOM),
                (GENESIS_ALLOCATION, IBC_USDC_DENOM),
            ],
        )?;

        keys.push(key);
    }

    let _cd = sh.push_dir(home_dir);

    find_and_replace_in_file!(
        sh,
        "config/config.toml",
        r#"timeout_commit = "5s""#  => r#"timeout_commit = "1s""#,
        r#"timeout_propose = "3s""# => r#"timeout_propose = "1s""#,
        "index_all_keys = false"    => "index_all_keys = true",
        "tcp://0.0.0.0:26656"       => "tcp://127.0.0.1:{p2p_port}",
        "tcp://127.0.0.1:26657"     => "tcp://127.0.0.1:{rpc_port}"
    );

    find_and_replace_in_file!(
        sh,
        "config/app.toml",
        "enable = false"                => "enable = true",
        "swagger = false"               => "swagger = true",
        "prometheus-retention-time = 0" => "prometheus-retention-time = 1000" ,
        r#"minimum-gas-prices = """#    =>
            r#"minimum-gas-prices = "0.0025{stake_denom},0.0025ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2""# ,
        "tcp://0.0.0.0:1317"            => "tcp://127.0.0.1:{rest_port}",
        r#"address = ":8080""#          => r#"address = ":{rosetta_port}""#
    );

    find_and_replace_in_file!(
        sh,
        "config/genesis.json",
        r#""denom": "stake""#      =>  r#""denom": "{stake_denom}""#,
        r#""mint_denom": "stake""# =>  r#""mint_denom": "{stake_denom}""#,
        r#""bond_denom": "stake""# =>  r#""bond_denom": "{stake_denom}""#
    );

    Ok(keys)
}

macro_rules! impl_path_fns {
        ($t:ident, $($path:ident),+) => {
            impl $t {
                $(
                    fn $path(&self) -> &Path {
                        self.$path.as_path()
                    }
                )+
            }
        }
    }

macro_rules! impl_clone_and_run {
    ($t:ident, $repo_url:expr, $repo_branch:expr) => {
        impl $t {
            fn clone_and_run<F>(&self, sh: &Shell, run_fn: F) -> Result<(), Error>
            where
                F: FnOnce(&Path) -> Result<(), Error>,
            {
                let src_path = self.src_path();
                let bin_path = self.bin_path();
                let repo_url = $repo_url;
                let repo_branch = $repo_branch;

                if !sh.path_exists(src_path) {
                    cmd!(
                        sh,
                        "git clone --depth 1 --branch {repo_branch} {repo_url} {src_path}"
                    )
                    .run()?;
                }

                let root = sh.current_dir();

                if !sh.path_exists(bin_path) {
                    let _cd = sh.push_dir(src_path);

                    run_fn(&root)?;
                }

                Ok(())
            }
        }
    };
}

macro_rules! impl_is_initialised {
    ($t:ident, $($path:ident),+) => {
        impl $t {
            fn is_initialized(&self, sh: &Shell) -> bool {
                [
                    $(self.$path(),)+
                ]
                .iter()
                .all(|path| sh.path_exists(path))
            }
        }
    }
}

macro_rules! impl_node_uri {
    ($t:ident, $port:expr) => {
        impl $t {
            fn node_uri(&self) -> NodeUri {
                let port = $port;
                format!("tcp://127.0.0.1:{port}").into()
            }
        }
    };
}

struct Handle {
    inner: Option<DuctHandle>,
    logfile_path: PathBuf,
}

impl_path_fns!(Handle, logfile_path);

#[derive(Clone, Copy, Debug)]
enum LogfileMode {
    Overwrite,
    Append,
}

impl Handle {
    fn try_from_duct_expression(
        sh: &Shell,
        expr: &DuctExpression,
        logfile_path: &Path,
        logfile_mode: LogfileMode,
    ) -> Result<Self, Error> {
        let home = make_abs_root!(sh);

        let logfile = match logfile_mode {
            LogfileMode::Overwrite => File::create(logfile_path)?,
            LogfileMode::Append => File::open(logfile_path)?,
        };

        let inner = expr
            .env("HOME", home)
            .stderr_to_stdout()
            .stdout_file(logfile)
            .start()?;

        Ok(Self {
            inner: Some(inner),
            logfile_path: logfile_path.to_owned(),
        })
    }

    fn wait(&mut self) -> Result<(), Error> {
        if let Some(inner) = self.inner.take() {
            inner.into_output()?;
        }
        Ok(())
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let Some(inner) = self.inner.take() else {
            return;
        };

        if let Err(err) = inner.kill() {
            let logfile_name = self
                .logfile_path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("unknown child process");

            error!("{logfile_name} encountered an error: {err}");
        }
    }
}

struct Neutrond {
    src_path: PathBuf,
    home_path: PathBuf,
    bin_path: PathBuf,
    logfile_path: PathBuf,
}

impl_path_fns!(Neutrond, src_path, home_path, bin_path, logfile_path);

impl_is_initialised!(Neutrond, src_path, home_path, bin_path);

impl_clone_and_run!(Neutrond, NTRN_REPO_URL, NTRN_REPO_BRANCH);

impl_node_uri!(Neutrond, NTRN_RPC_PORT);

impl Neutrond {
    fn new(sh: &Shell) -> Self {
        Self {
            src_path: make_abs_path!(sh, NTRN_REPO_CLONE_DIR),
            home_path: make_abs_path!(sh, NTRN_CHAIN_HOME_DIR),
            bin_path: make_abs_path!(sh, NTRN_BIN_PATH),
            logfile_path: make_abs_path!(sh, NTRN_LOGFILE),
        }
    }

    fn cli<'a>(&self, sh: &'a Shell) -> Cmd<'a> {
        let bin_path = self.bin_path();
        let home_path = self.home_path();

        cmd!(sh, "{bin_path} --home {home_path}").into()
    }

    fn init(&self, sh: &Shell) -> Result<(), Error> {
        self.clone_and_run(sh, |root| {
            cmd!(sh, "make install-test-binary")
                .env(
                    "GOPATH",
                    concat_paths!(root.to_owned(), home_path_prefix!()),
                )
                // make go module cache not break rm -r
                // https://go.dev/doc/go1.14#go-command
                .env("GOFLAGS", "-modcacherw")
                .run()
                .map_err(Error::from)
        })?;

        let bin_path = self.bin_path();

        let home_path = self.home_path();

        sh.remove_path(home_path).ok();

        init_chain(
            sh,
            || self.cli(sh),
            home_path,
            InitParams {
                chain_id: NTRN_CHAIN_ID,
                stake_denom: NTRN_CHAIN_DENOM,
                p2p_port: NTRN_P2P_PORT,
                rpc_port: NTRN_RPC_PORT,
                rest_port: NTRN_REST_PORT,
                rosetta_port: NTRN_ROSETTA_PORT,
            },
        )?;

        cmd!(sh, "{bin_path} add-consumer-section --home {home_path}").run()?;

        let _cd = sh.push_dir(home_path);

        find_and_replace_in_file!(
            sh,
            "config/genesis.json",
            r#""allow_messages": []"#                                 => r#""allow_messages": ["*"]"#,
            r#""signed_blocks_window": "100""#                        => r#""signed_blocks_window": "140000""#,
            r#""min_signed_per_window": "0.500000000000000000""#      => r#""min_signed_per_window": "0.050000000000000000""#,
            r#""slash_fraction_double_sign": "0.050000000000000000""# => r#""slash_fraction_double_sign": "0.010000000000000000""#,
            r#""slash_fraction_downtime": "0.010000000000000000""#    => r#""slash_fraction_downtime": "0.000100000000000000""#,
            r#""minimum_gas_prices": []"# =>
                r#""minimum_gas_prices": [
                    {{"denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2","amount":"0"}},
                    {{"denom":"untrn","amount":"0"}}
                ]"#
        );

        Ok(())
    }

    fn start(&self, sh: &Shell) -> Result<Handle, Error> {
        let expr = duct::cmd!(
            self.bin_path(),
            "start",
            "--log_level",
            "trace",
            "--log_format",
            "json",
            "--home",
            self.home_path(),
            "--pruning=nothing",
            format!(r#"--grpc.address=127.0.0.1:{NTRN_GRPC_PORT}"#),
            format!(r#"--grpc-web.address=127.0.0.1:{NTRN_GRPC_WEB_PORT}"#),
            "--trace"
        );

        Handle::try_from_duct_expression(sh, &expr, self.logfile_path(), LogfileMode::Overwrite)
    }
}

struct Gaiad {
    src_path: PathBuf,
    home_path: PathBuf,
    bin_path: PathBuf,
    logfile_path: PathBuf,
}

impl_path_fns!(Gaiad, src_path, home_path, bin_path, logfile_path);

impl_is_initialised!(Gaiad, src_path, home_path, bin_path);

impl_clone_and_run!(Gaiad, GAIA_REPO_URL, GAIA_REPO_BRANCH);

impl_node_uri!(Gaiad, GAIA_RPC_PORT);

impl Gaiad {
    fn new(sh: &Shell) -> Self {
        Self {
            src_path: make_abs_path!(sh, GAIA_REPO_CLONE_DIR),
            home_path: make_abs_path!(sh, GAIA_CHAIN_HOME_DIR),
            bin_path: make_abs_path!(sh, GAIA_BIN_PATH),
            logfile_path: make_abs_path!(sh, GAIA_LOGFILE),
        }
    }

    fn cli<'a>(&self, sh: &'a Shell) -> Cmd<'a> {
        let bin_path = self.bin_path();
        let home_path = self.home_path();

        cmd!(sh, "{bin_path} --home {home_path}").into()
    }

    fn init(&self, sh: &Shell) -> Result<(), Error> {
        self.clone_and_run(sh, |root| {
            find_and_replace_in_file!(
                sh,
                "Makefile",
                "$(BUILD_TARGETS): check_version go.sum $(BUILDDIR)/" => "$(BUILD_TARGETS): go.sum $(BUILDDIR)/"
            );

            cmd!(sh, "make install")
                .env("GOPATH", concat_paths!(root.to_owned(), home_path_prefix!()))
                // make go module cache not break rm -r
                // https://go.dev/doc/go1.14#go-command
                .env("GOFLAGS", "-modcacherw") 
                .run()
                .map_err(Error::from)
        })?;

        let home_path = self.home_path();

        sh.remove_path(home_path).ok();

        let keys = init_chain(
            sh,
            || self.cli(sh),
            home_path,
            InitParams {
                chain_id: GAIA_CHAIN_ID,
                stake_denom: GAIA_CHAIN_DENOM,
                p2p_port: GAIA_P2P_PORT,
                rpc_port: GAIA_RPC_PORT,
                rest_port: GAIA_REST_PORT,
                rosetta_port: GAIA_ROSETTA_PORT,
            },
        )?;

        let _cd = sh.push_dir(home_path);

        find_and_replace_in_file!(
            sh,
            "config/genesis.json",
            r#""allow_messages": []"# =>
                r#""allow_messages": [
                    "/cosmos.bank.v1beta1.MsgSend",
                    "/cosmos.staking.v1beta1.MsgDelegate",
                    "/cosmos.staking.v1beta1.MsgUndelegate"
                ]"#
        );

        self.cli(sh)
            .gentx(&keys[3], 7_000_000_000, GAIA_CHAIN_DENOM, GAIA_CHAIN_ID)?;

        self.cli(sh).collect_gentx()?;

        Ok(())
    }

    fn start(&self, sh: &Shell) -> Result<Handle, Error> {
        let expr = duct::cmd!(
            self.bin_path(),
            "start",
            "--log_level",
            "trace",
            "--log_format",
            "json",
            "--home",
            self.home_path(),
            "--pruning=nothing",
            format!(r#"--grpc.address=127.0.0.1:{GAIA_GRPC_PORT}"#),
            format!(r#"--grpc-web.address=127.0.0.1:{GAIA_GRPC_WEB_PORT}"#),
            "--trace"
        );

        Handle::try_from_duct_expression(sh, &expr, self.logfile_path(), LogfileMode::Overwrite)
    }
}

struct Hermesd {
    home_path: PathBuf,
    config_file_path: PathBuf,
    bin_path: PathBuf,
    logfile_path: PathBuf,
}

impl_path_fns!(Hermesd, home_path, config_file_path, bin_path, logfile_path);

impl_is_initialised!(Hermesd, bin_path, home_path);

impl Hermesd {
    fn new(sh: &Shell) -> Self {
        Self {
            home_path: make_abs_path!(sh, HERMES_HOME_DIR),
            config_file_path: make_abs_path!(sh, HERMES_HOME_DIR, HERMES_CONFIG_FILE),
            bin_path: make_abs_path!(sh, HERMES_BIN_PATH),
            logfile_path: make_abs_path!(sh, HERMES_LOGFILE),
        }
    }

    fn cli<'a>(&self, sh: &'a Shell) -> ShellCmd<'a> {
        let bin_path = self.bin_path();
        let config_file = self.config_file_path();

        cmd!(sh, "{bin_path} --config {config_file}")
    }

    fn init(&self, sh: &Shell, neutrond: &Neutrond) -> Result<(), Error> {
        if !sh.path_exists(self.bin_path()) {
            let root = make_abs_root!(sh);
            cmd!(
                sh,
                "cargo install {HERMES_CRATE} --bin {HERMES_CRATE_BIN} --version {HERMES_CRATE_VERSION} --locked --root {root}"
            )
            .run()?;
        }

        let copy_config_src =
            concat_paths!(neutrond.src_path().to_owned(), HERMES_COPY_CONFIG_PATH);

        sh.remove_path(self.home_path()).ok();

        sh.create_dir(self.home_path())?;

        sh.copy_file(copy_config_src, self.config_file_path())?;

        let mnemonic1_file = concat_paths!(self.home_path().to_owned(), "mnemonic1.txt");

        let mnemonic2_file = concat_paths!(self.home_path().to_owned(), "mnemonic2.txt");

        sh.write_file(&mnemonic1_file, RLY_MNEMONIC_1)?;

        sh.write_file(&mnemonic2_file, RLY_MNEMONIC_2)?;

        self.cli(sh)
            .args(["keys", "delete", "--chain", NTRN_CHAIN_ID, "--all"])
            .env("HOME", make_abs_root!(sh))
            .run()?;

        self.cli(sh)
            .args([
                "keys",
                "add",
                "--key-name",
                "testkey_1",
                "--chain",
                NTRN_CHAIN_ID,
                "--mnemonic-file",
            ])
            .env("HOME", make_abs_root!(sh))
            .arg(&mnemonic1_file)
            .run()?;

        self.cli(sh)
            .args(["keys", "delete", "--chain", GAIA_CHAIN_ID, "--all"])
            .env("HOME", make_abs_root!(sh))
            .run()?;

        self.cli(sh)
            .args([
                "keys",
                "add",
                "--key-name",
                "testkey_2",
                "--chain",
                GAIA_CHAIN_ID,
                "--mnemonic-file",
            ])
            .env("HOME", make_abs_root!(sh))
            .arg(&mnemonic2_file)
            .run()?;

        Ok(())
    }

    fn start(&self, sh: &Shell) -> Result<Handle, Error> {
        let bin_path = self.bin_path();

        let config_path = self.config_file_path();

        // Why do you need this Hermes?
        std::thread::sleep(std::time::Duration::from_secs(5));

        Handle::try_from_duct_expression(
            sh,
            &duct::cmd!(
                bin_path,
                "--config",
                config_path,
                "create",
                "connection",
                "--a-chain",
                NTRN_CHAIN_ID,
                "--b-chain",
                GAIA_CHAIN_ID,
            ),
            self.logfile_path(),
            LogfileMode::Overwrite,
        )?
        .wait()?;

        Handle::try_from_duct_expression(
            sh,
            &duct::cmd!(
                bin_path,
                "--config",
                config_path,
                "create",
                "channel",
                "--a-chain",
                NTRN_CHAIN_ID,
                "--a-connection",
                "connection-0",
                "--a-port",
                "transfer",
                "--b-port",
                "transfer",
            ),
            self.logfile_path(),
            LogfileMode::Append,
        )?
        .wait()?;

        Handle::try_from_duct_expression(
            sh,
            &duct::cmd!(bin_path, "--config", config_path, "start"),
            self.logfile_path(),
            LogfileMode::Append,
        )
    }
}

struct IcqRlyd {
    src_path: PathBuf,
    bin_path: PathBuf,
    db_path: PathBuf,
    logfile_path: PathBuf,
}

impl_path_fns!(IcqRlyd, src_path, bin_path, db_path, logfile_path);

impl_is_initialised!(IcqRlyd, src_path, bin_path);

impl_clone_and_run!(IcqRlyd, ICQ_RLY_REPO_URL, ICQ_RLY_REPO_BRANCH);

impl IcqRlyd {
    fn new(sh: &Shell) -> Self {
        Self {
            src_path: make_abs_path!(sh, ICQ_RLY_REPO_CLONE_DIR),
            bin_path: make_abs_path!(sh, ICQ_RLY_BIN_PATH),
            db_path: make_abs_path!(sh, ICQ_RLY_DB_PATH),
            logfile_path: make_abs_path!(sh, ICQ_RLY_LOGFILE),
        }
    }

    fn init(&self, sh: &Shell) -> Result<(), Error> {
        self.clone_and_run(sh, |root| {
            cmd!(sh, "make install")
                .env(
                    "GOPATH",
                    concat_paths!(root.to_owned(), home_path_prefix!()),
                )
                // make go module cache not break rm -r
                // https://go.dev/doc/go1.14#go-command
                .env("GOFLAGS", "-modcacherw")
                .run()
                .map_err(Error::from)
        })
    }

    fn start(&self, sh: &Shell, neutrond: &Neutrond, gaiad: &Gaiad) -> Result<Handle, Error> {
        macro_rules! set_env_vars {
            ($cmd:ident, $($key:literal = $value:literal),+) => {{
                let vars = [
                    $(($key, format!($value))),+
                ];

                let mut cmd = $cmd;

                for (k, v) in vars {
                    cmd = cmd.env(k, v);
                }

                cmd
            }}
        }

        let cmd = duct::cmd!(self.bin_path(), "start");

        let cmd = set_env_vars!(
            cmd,
            "RELAYER_NEUTRON_CHAIN_CHAIN_PREFIX" = "neutron",
            "RELAYER_NEUTRON_CHAIN_RPC_ADDR" = "tcp://127.0.0.1:{NTRN_RPC_PORT}",
            "RELAYER_NEUTRON_CHAIN_REST_ADDR" = "http://127.0.0.1:{NTRN_REST_PORT}",
            "RELAYER_NEUTRON_CHAIN_CHAIN_ID" = "test-1",
            "RELAYER_NEUTRON_CHAIN_GAS_PRICES" = "0.5untrn",
            "RELAYER_NEUTRON_CHAIN_SIGN_KEY_NAME" = "local3",
            "RELAYER_NEUTRON_CHAIN_TIMEOUT" = "1000s",
            "RELAYER_NEUTRON_CHAIN_GAS_ADJUSTMENT" = "2.0",
            "RELAYER_NEUTRON_CHAIN_TX_BROADCAST_TYPE" = "BroadcastTxCommit",
            "RELAYER_NEUTRON_CHAIN_CONNECTION_ID" = "connection-0",
            "RELAYER_NEUTRON_CHAIN_CLIENT_ID" = "07-tendermint-0",
            "RELAYER_NEUTRON_CHAIN_DEBUG" = "true",
            "RELAYER_NEUTRON_CHAIN_KEY" = "local1",
            "RELAYER_NEUTRON_CHAIN_ACCOUNT_PREFIX" = "neutron",
            "RELAYER_NEUTRON_CHAIN_KEYRING_BACKEND" = "test",
            "RELAYER_NEUTRON_CHAIN_OUTPUT_FORMAT" = "json",
            "RELAYER_NEUTRON_CHAIN_SIGN_MODE_STR" = "direct",
            "RELAYER_NEUTRON_CHAIN_ALLOW_KV_CALLBACKS" = "true",
            "RELAYER_TARGET_CHAIN_RPC_ADDR" = "tcp://127.0.0.1:{GAIA_RPC_PORT}",
            "RELAYER_TARGET_CHAIN_CHAIN_ID" = "test-2",
            "RELAYER_TARGET_CHAIN_GAS_PRICES" = "0.5uatom",
            "RELAYER_TARGET_CHAIN_TIMEOUT" = "1000s",
            "RELAYER_TARGET_CHAIN_GAS_ADJUSTMENT" = "1.0",
            "RELAYER_TARGET_CHAIN_CONNECTION_ID" = "connection-0",
            "RELAYER_TARGET_CHAIN_CLIENT_ID" = "07-tendermint-0",
            "RELAYER_TARGET_CHAIN_DEBUG" = "true",
            "RELAYER_TARGET_CHAIN_KEYRING_BACKEND" = "test",
            "RELAYER_TARGET_CHAIN_OUTPUT_FORMAT" = "json",
            "RELAYER_TARGET_CHAIN_SIGN_MODE_STR" = "direct",
            "RELAYER_REGISTRY_ADDRESSES" = "",
            "RELAYER_ALLOW_TX_QUERIES" = "true",
            "RELAYER_ALLOW_KV_CALLBACKS" = "true",
            "RELAYER_MIN_KV_UPDATE_PERIOD" = "1",
            "RELAYER_QUERIES_TASK_QUEUE_CAPACITY" = "10000",
            "RELAYER_CHECK_SUBMITTED_TX_STATUS_DELAY" = "10s",
            "RELAYER_WEBSERVER_PORT" = "127.0.0.1:9999"
        )
        .env("RELAYER_NEUTRON_CHAIN_HOME_DIR", neutrond.home_path())
        .env("RELAYER_TARGET_CHAIN_HOME_DIR", gaiad.home_path())
        .env("RELAYER_STORAGE_PATH", self.db_path());

        Handle::try_from_duct_expression(sh, &cmd, self.logfile_path(), LogfileMode::Overwrite)
    }
}

pub struct Local {
    neutrond: Neutrond,
    gaiad: Gaiad,
    hermesd: Hermesd,
    icq_rlyd: IcqRlyd,
}

impl Local {
    fn new(sh: &Shell) -> Self {
        Self {
            neutrond: Neutrond::new(sh),
            gaiad: Gaiad::new(sh),
            hermesd: Hermesd::new(sh),
            icq_rlyd: IcqRlyd::new(sh),
        }
    }

    fn init(&self, sh: &Shell) -> Result<(), Error> {
        if self.neutrond.is_initialized(sh)
            && self.gaiad.is_initialized(sh)
            && self.hermesd.is_initialized(sh)
            && self.icq_rlyd.is_initialized(sh)
        {
            return Ok(());
        }

        self.neutrond.init(sh)?;

        self.gaiad.init(sh)?;

        self.hermesd.init(sh, &self.neutrond)?;

        self.icq_rlyd.init(sh)?;

        Ok(())
    }

    fn start(&self, sh: &Shell) -> Result<Handles, Error> {
        info!("starting neutron");
        let ntrn = self.neutrond.start(sh)?;

        info!("starting gaia");
        let gaia = self.gaiad.start(sh)?;

        info!("waiting for neutron blocks");
        wait_for_blocks_fn(|| Ok(self.neutrond.cli(sh)), &self.neutrond.node_uri())?;

        info!("waiting for gaia blocks");
        wait_for_blocks_fn(|| Ok(self.gaiad.cli(sh)), &self.gaiad.node_uri())?;

        info!("starting hermes");
        let hermes = self.hermesd.start(sh)?;

        info!("starting ICQ relayer");
        let icq_rly = self.icq_rlyd.start(sh, &self.neutrond, &self.gaiad)?;

        Ok(Handles {
            ntrn,
            _gaia: gaia,
            _icq_rly: icq_rly,
            _hermes: hermes,
        })
    }
}

impl Initialize for Local {
    type Instance = Instance<Local>;

    fn initialize(sh: &Shell) -> Result<Instance<Self>, Error> {
        let network = Local::new(sh);

        network.init(sh)?;

        let keys = network.neutrond.cli(sh).list_keys(KeyringBackend::Test)?;

        Ok(Instance { keys, network })
    }
}

impl Cli for Instance<Local> {
    fn cli<'a>(&self, sh: &'a Shell) -> Result<Cmd<'a>, Error> {
        Ok(self.network().neutrond.cli(sh))
    }
}

pub struct Handles {
    ntrn: Handle,
    _gaia: Handle,
    _icq_rly: Handle,
    _hermes: Handle,
}

fn follow_file(path: &Path) -> Result<(), Error> {
    let keep_running = Arc::new(AtomicBool::new(true));

    ctrlc::set_handler({
        let keep_running = keep_running.clone();
        move || keep_running.store(false, Ordering::Relaxed)
    })?;

    let file = File::open(path)?;

    let mut reader = BufReader::new(file);

    let mut line = String::new();

    while keep_running.load(Ordering::Relaxed) {
        while reader.read_line(&mut line)? > 0 {
            eprint!("{line}");
            line.clear();
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    Ok(())
}

impl IntoForeground for Handles {
    fn into_foreground(self) -> Result<(), Error> {
        info!(
            "bringing nuetrond to the foreground - following {}",
            self.ntrn.logfile_path().display()
        );
        follow_file(self.ntrn.logfile_path())
    }
}

impl StartLocal for Instance<Local> {
    type Handle<'shell> = Handles;

    fn start_local<'shell>(&self, sh: &'shell Shell) -> Result<Self::Handle<'shell>, Error> {
        self.network().start(sh)
    }
}

impl Node for Instance<Local> {
    fn node_uri(&self, _sh: &Shell) -> Result<NodeUri, Error> {
        Ok(self.network().neutrond.node_uri())
    }

    fn chain_id(&self) -> ChainId {
        ChainId::from(NTRN_CHAIN_ID.to_owned())
    }
}

impl Clean for Local {
    fn clean_state(sh: &Shell) -> Result<(), Error> {
        sh.remove_path(make_abs_path!(sh, NTRN_CHAIN_HOME_DIR)).ok();
        sh.remove_path(make_abs_path!(sh, GAIA_CHAIN_HOME_DIR)).ok();
        sh.remove_path(make_abs_path!(sh, HERMES_HOME_DIR)).ok();
        sh.remove_path(make_abs_path!(sh, ICQ_RLY_DB_PATH)).ok();
        Ok(())
    }

    fn clean_all(sh: &Shell) -> Result<(), Error> {
        sh.remove_path(make_abs_root!(sh)).ok();
        Ok(())
    }
}

impl GasPrices for Instance<Local> {
    fn low_gas_price(&self) -> GasPrice {
        GasPrice::new(0.01, NTRN_CHAIN_DENOM)
    }

    fn medium_gas_price(&self) -> GasPrice {
        GasPrice::new(0.02, NTRN_CHAIN_DENOM)
    }

    fn high_gas_price(&self) -> GasPrice {
        GasPrice::new(0.04, NTRN_CHAIN_DENOM)
    }
}
