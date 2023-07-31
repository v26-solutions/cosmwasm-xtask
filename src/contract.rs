use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use xshell::Shell;

use crate::{
    cli::{wait_for_tx, CodeId, Contract, CwExecuteResponse, TxData},
    key::Key,
    network::Network,
    Error,
};

pub enum Cmd<Msg> {
    Store(PathBuf),
    Instantiate {
        code_id: CodeId,
        label: String,
        msg: Msg,
    },
    Execute {
        contract: Contract,
        msg: Msg,
    },
}

pub struct Tx<Msg, Response> {
    cmd: Cmd<Msg>,
    gas_units: u128,
    amount: Option<(u128, String)>,
    _r: PhantomData<Response>,
}

impl<Msg, Response> Tx<Msg, Response>
where
    Response: prost::Message + Default,
    Msg: Serialize,
{
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
            Cmd::Store(path) => {
                println!("Storing contract bytecode: {}", path.as_path().display());
                cmd.wasm_store(path)
            }
            Cmd::Instantiate {
                code_id,
                label,
                msg,
            } => {
                let msg_json = serde_json::to_string_pretty(&msg)?;
                println!("Initialising {label} with code id {code_id} with message:\n{msg_json}");
                cmd.wasm_init(code_id, &label, &msg_json, None)
            }
            Cmd::Execute { contract, msg } => {
                let msg_json = serde_json::to_string_pretty(&msg)?;
                println!("Executing {contract} with message:\n{msg_json}",);
                cmd.wasm_exec(&contract, &msg_json)
            }
        };

        let cmd = if let Some((amount, denom)) = self.amount {
            cmd.amount(amount, &denom)
        } else {
            cmd
        };

        let tx_id = cmd.execute(&gas)?;

        println!("TX: {tx_id}");

        wait_for_tx(sh, network, &tx_id)?
            .decode()
            .map(TxData::into_data)
    }
}

/// Construct a tx to store some WASM bytecode on the `network`, responds with the code ID.
pub fn store<P>(wasm_path: P) -> Tx<(), CodeId>
where
    P: AsRef<Path>,
{
    Tx {
        cmd: Cmd::Store(wasm_path.as_ref().to_path_buf()),
        gas_units: 100_000_000,
        amount: None,
        _r: PhantomData,
    }
}

/// Construct a tx to instantiate a contract with the given `code_id` on the `network` with `msg`, responds with the contract address.
pub fn instantiate<Msg>(code_id: CodeId, label: &str, msg: Msg) -> Tx<Msg, Contract> {
    Tx {
        cmd: Cmd::Instantiate {
            code_id,
            label: label.to_owned(),
            msg,
        },
        gas_units: 100_000_000,
        amount: None,
        _r: PhantomData,
    }
}

/// Construct a command to tx a `contract` with a `msg`, responding with the response bytes.
pub fn execute<Msg>(contract: &Contract, msg: Msg) -> Tx<Msg, CwExecuteResponse> {
    Tx {
        cmd: Cmd::Execute {
            contract: contract.clone(),
            msg,
        },
        gas_units: 100_000_000,
        amount: None,
        _r: PhantomData,
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

    println!("Querying {contract} with message:\n{msg_json}",);

    let res_json = network
        .cli(sh)?
        .query(&node_uri)
        .wasm_smart(contract, &msg_json)?;

    serde_json::from_str::<QueryData<Response>>(&res_json)
        .map(|res| res.data)
        .map_err(Error::from)
}
