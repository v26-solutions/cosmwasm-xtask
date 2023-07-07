use std::path::Path;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use xshell::Shell;

use crate::{
    cli::{wait_for_tx, CodeId, Contract, CwExecuteResponse, TxData},
    key::Key,
    network::Network,
    Error,
};

/// Store some WASM bytecode on the `network`, returning the code ID.
///
/// # Errors
///
/// This function will return an error if:
/// - Command execution fails
/// - The response from the node contains an error
/// - Decoding the `TxData` fails
pub fn store<P>(
    sh: &Shell,
    network: &dyn Network,
    from: &Key,
    wasm_path: P,
    gas_units: Option<u128>,
) -> Result<CodeId, Error>
where
    P: AsRef<Path>,
{
    let gas = network
        .medium_gas_price()
        .units(gas_units.unwrap_or(100_000_000));

    let chain_id = network.chain_id();

    let node_uri = network.node_uri(sh)?;

    println!(
        "Storing contract bytecode: {}",
        wasm_path.as_ref().display()
    );

    let tx_id = network
        .cli(sh)?
        .tx(from, &chain_id, &node_uri)
        .wasm_store(wasm_path)
        .execute(&gas)?;

    println!("Store TX: {tx_id}");

    wait_for_tx(sh, network, &tx_id)?
        .decode()
        .map(TxData::into_data)
}

/// Instantiate a contract with the given `code_id` on the `network` with `msg`, returning the contract address.
///
/// # Errors
///
/// This function will return an error if:
/// - Command execution fails
/// - The response from the node contains an error
/// - Decoding the `TxData` fails
pub fn instantiate<Msg>(
    sh: &Shell,
    network: &dyn Network,
    code_id: CodeId,
    owner: &Key,
    label: &str,
    msg: &Msg,
    gas_units: Option<u128>,
) -> Result<Contract, Error>
where
    Msg: Serialize,
{
    let gas = network
        .medium_gas_price()
        .units(gas_units.unwrap_or(100_000_000));

    let chain_id = network.chain_id();

    let node_uri = network.node_uri(sh)?;

    let msg_json = serde_json::to_string_pretty(msg)?;

    println!("Initialising {label} with code id {code_id} with message:\n{msg_json}",);

    let tx_id = network
        .cli(sh)?
        .tx(owner, &chain_id, &node_uri)
        .wasm_init(code_id, label, &msg_json, None)
        .execute(&gas)?;

    println!("Initialise TX: {tx_id}");

    wait_for_tx(sh, network, &tx_id)?
        .decode()
        .map(TxData::into_data)
}

/// Execute a `contract` on the `network` with `msg`, returning the response bytes.
///
/// # Errors
///
/// This function will return an error if:
/// - Command execution fails
/// - The response from the node contains an error
/// - Decoding the `TxData` fails
pub fn execute<Msg>(
    sh: &Shell,
    network: &dyn Network,
    contract: &Contract,
    from: &Key,
    msg: &Msg,
    gas_units: Option<u128>,
) -> Result<CwExecuteResponse, Error>
where
    Msg: Serialize,
{
    let gas = network
        .medium_gas_price()
        .units(gas_units.unwrap_or(100_000_000));

    let chain_id = network.chain_id();

    let node_uri = network.node_uri(sh)?;

    let msg_json = serde_json::to_string_pretty(msg)?;

    println!("Executing {contract} with message:\n{msg_json}",);

    let tx_id = network
        .cli(sh)?
        .tx(from, &chain_id, &node_uri)
        .wasm_exec(contract, &msg_json)
        .execute(&gas)?;

    println!("Execute TX: {tx_id}");

    wait_for_tx(sh, network, &tx_id)?
        .decode()
        .map(TxData::into_data)
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
