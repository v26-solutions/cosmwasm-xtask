use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

use log::debug;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use xshell::Shell;

use crate::{
    cli::{wait_for_tx, CodeId, Contract, CwExecuteResponse, ReadyTxCmd, TxData},
    key::Key,
    network::Network,
    Error,
};

pub struct Store {
    path: PathBuf,
}

pub struct Instantiate {
    code_id: CodeId,
    label: String,
    admin: Option<String>,
}

pub struct Execute {
    contract: Contract,
}

pub enum Cmd<Msg> {
    Store(Store),
    Instantiate { opts: Instantiate, msg: Msg },
    Execute { opts: Execute, msg: Msg },
}

type PreExecuteBuildHook = Box<dyn for<'a> FnOnce(ReadyTxCmd<'a>) -> ReadyTxCmd<'a>>;

pub struct Tx<Opts, Msg, Response> {
    cmd: Cmd<Msg>,
    gas_units: u128,
    amount: Option<(u128, String)>,
    pre_execute_hook: Option<PreExecuteBuildHook>,
    _r: PhantomData<Response>,
    _opts: PhantomData<Opts>,
}

impl<Msg, Response> Tx<Instantiate, Msg, Response> {
    fn opts_mut(&mut self) -> &mut Instantiate {
        match &mut self.cmd {
            Cmd::Instantiate { opts, .. } => opts,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn admin(mut self, admin: &str) -> Self {
        self.opts_mut().admin = Some(admin.to_owned());
        self
    }
}

impl<Opts, Msg, Response> Tx<Opts, Msg, Response> {
    #[must_use]
    pub fn gas(mut self, units: u128) -> Self {
        self.gas_units = units;
        self
    }

    #[must_use]
    pub fn amount(mut self, amount: u128, denom: &str) -> Self {
        self.amount = Some((amount, denom.to_owned()));
        self
    }

    #[must_use]
    pub fn pre_execute_hook<F>(mut self, f: F) -> Self
    where
        F: for<'a> FnOnce(ReadyTxCmd<'a>) -> ReadyTxCmd<'a> + 'static,
    {
        self.pre_execute_hook = Some(Box::new(f));
        self
    }
}

impl<Opts, Msg, Response> Tx<Opts, Msg, Response>
where
    Response: prost::Message + Default,
    Msg: Serialize,
{
    /// Send the tx, wait for it to be included in a block, then return the decoded `Response`
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Command execution fails
    /// - The response from the node contains an error
    /// - Decoding the `TxData` fails
    pub fn send(self, sh: &Shell, network: &dyn Network, from: &Key) -> Result<Response, Error> {
        let gas = network.medium_gas_price().units(self.gas_units);

        let chain_id = network.chain_id();

        let node_uri = network.node_uri(sh)?;

        let cmd = network.cli(sh)?.tx(from, &chain_id, &node_uri);

        let cmd = match self.cmd {
            Cmd::Store(Store { path }) => {
                debug!("Storing contract bytecode: {}", path.as_path().display());
                cmd.wasm_store(path)
            }
            Cmd::Instantiate {
                opts:
                    Instantiate {
                        code_id,
                        label,
                        admin,
                    },
                msg,
            } => {
                let msg_json = serde_json::to_string_pretty(&msg)?;
                debug!("Initialising {label} with code id {code_id} with message:\n{msg_json}");

                cmd.wasm_init(code_id, &label, &msg_json, admin.as_deref())
            }
            Cmd::Execute {
                opts: Execute { contract },
                msg,
            } => {
                let msg_json = serde_json::to_string_pretty(&msg)?;
                debug!("Executing {contract} with message:\n{msg_json}",);
                cmd.wasm_exec(&contract, &msg_json)
            }
        };

        let cmd = if let Some((amount, denom)) = self.amount {
            cmd.amount(amount, &denom)
        } else {
            cmd
        };

        let tx_id = cmd.execute(&gas)?;

        debug!("TX: {tx_id}");

        wait_for_tx(sh, network, &tx_id)?
            .decode()
            .map(TxData::into_data)
    }
}

/// Construct a tx to store some WASM bytecode on the `network`, responds with the code ID.
pub fn store<P>(wasm_path: P) -> Tx<Store, (), CodeId>
where
    P: AsRef<Path>,
{
    Tx {
        cmd: Cmd::Store(Store {
            path: wasm_path.as_ref().to_path_buf(),
        }),
        gas_units: 100_000_000,
        amount: None,
        pre_execute_hook: None,
        _r: PhantomData,
        _opts: PhantomData,
    }
}

/// Get a predictable address for an instantiated `code_id` on the `network` with the given `creator` & `salt`
///
/// # Errors
///
/// This function will return an error if:
/// - Command execution fails
pub fn predict_adddress(
    sh: &Shell,
    network: &dyn Network,
    code_id: CodeId,
    creator: &Key,
    salt: &str,
) -> Result<String, Error> {
    let node_uri = network.node_uri(sh)?;
    let code_info = network.cli(sh)?.query(&node_uri).code_info(code_id)?;
    network
        .cli(sh)?
        .build_address(&code_info.data_hash, creator, salt)
}

/// Construct a tx to instantiate a contract with the given `code_id` on the `network` with `msg`, responds with the contract address.
pub fn instantiate<Msg>(code_id: CodeId, label: &str, msg: Msg) -> Tx<Instantiate, Msg, Contract> {
    Tx {
        cmd: Cmd::Instantiate {
            opts: Instantiate {
                code_id,
                label: label.to_owned(),
                admin: None,
            },
            msg,
        },
        gas_units: 100_000_000,
        amount: None,
        pre_execute_hook: None,
        _r: PhantomData,
        _opts: PhantomData,
    }
}

/// Construct a command to tx a `contract` with a `msg`, responding with the response bytes.
pub fn execute<Msg>(contract: &Contract, msg: Msg) -> Tx<Execute, Msg, CwExecuteResponse> {
    Tx {
        cmd: Cmd::Execute {
            opts: Execute {
                contract: contract.clone(),
            },
            msg,
        },
        gas_units: 100_000_000,
        amount: None,
        pre_execute_hook: None,
        _r: PhantomData,
        _opts: PhantomData,
    }
}

/// Query a `contract` on the `network` with `msg`, returning the response.
///
/// # Errors
///
/// This function will return an error if:
/// - Command execution fails
/// - The response from the node contains an error
/// - JSON deserialisation fails
pub fn query<Msg, Response>(
    sh: &Shell,
    network: &dyn Network,
    contract: &Contract,
    msg: &Msg,
) -> Result<Response, Error>
where
    Msg: Serialize,
    Response: DeserializeOwned,
{
    #[derive(Deserialize)]
    struct QueryData<T> {
        data: T,
    }

    let node_uri = network.node_uri(sh)?;

    let msg_json = serde_json::to_string_pretty(msg)?;

    debug!("Querying {contract} with message:\n{msg_json}",);

    let res_json = network
        .cli(sh)?
        .query(&node_uri)
        .wasm_smart(contract, &msg_json)?;

    serde_json::from_str::<QueryData<Response>>(&res_json)
        .map(|res| res.data)
        .map_err(Error::from)
}
