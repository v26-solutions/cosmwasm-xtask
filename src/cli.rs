use std::path::Path;

use derive_more::{Display, From, FromStr};
use prost::Message;
use serde::{de::DeserializeOwned, Deserialize};
use serde_aux::prelude::*;
use xshell::{Cmd as ShellCmd, Shell};

use crate::{
    key::{Key, KeyringBackend, Raw},
    network::{gas::Gas, ChainId, Network, NodeUri},
    Error,
};

pub trait Cli {
    /// Generate a Cmd builder
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementation.
    fn cli<'a>(&self, sh: &'a Shell) -> Result<Cmd<'a>, Error>;
}

#[derive(From)]
pub struct Cmd<'a>(ShellCmd<'a>);

pub struct BuildTxCmd<'a> {
    from: &'a Key,
    chain_id: &'a ChainId,
    node: &'a NodeUri,
    cmd: ShellCmd<'a>,
}

pub struct ReadyTxCmd<'a> {
    pub(crate) cmd: ShellCmd<'a>,
}

pub struct QueryCmd<'a> {
    cmd: ShellCmd<'a>,
}

#[derive(From, Display, Debug, Clone)]
pub struct TxId(String);

impl TxId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<'a> Cmd<'a> {
    /// List the keys associated with the given `backend`.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    /// - JSON deserialisation fails
    pub fn list_keys(self, backend: KeyringBackend) -> Result<Vec<Key>, Error> {
        let raw_keys: Vec<Raw> = self
            .0
            .args([
                "keys",
                "list",
                "--keyring-backend",
                backend.as_str(),
                "--output",
                "json",
            ])
            .output()
            .map_err(Error::from)
            .and_then(|out| serde_json::from_slice(&out.stdout).map_err(Error::from))?;

        let keys = raw_keys
            .into_iter()
            .map(|raw_key| raw_key.with_backend(backend))
            .collect();

        Ok(keys)
    }

    /// Add a key to be associated with the given `backend`.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    /// - JSON deserialisation fails
    pub fn add_key(self, name: &str, backend: KeyringBackend) -> Result<Key, Error> {
        self.0
            .args([
                "keys",
                "add",
                name,
                "--keyring-backend",
                backend.as_str(),
                "--output",
                "json",
            ])
            .read()
            .map_err(Error::from)
            .and_then(|out| {
                serde_json::from_str::<Raw>(&out)
                    .map(|raw_key| raw_key.with_backend(backend))
                    .map_err(Error::from)
            })
    }

    /// Recover a key with mnemonic to be associated with the given `backend`.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    /// - JSON deserialisation fails
    pub fn recover_key(
        self,
        name: &str,
        mnenomic: &str,
        backend: KeyringBackend,
    ) -> Result<Key, Error> {
        self.0
            .args([
                "keys",
                "add",
                name,
                "--keyring-backend",
                backend.as_str(),
                "--recover",
                "--output",
                "json",
            ])
            .stdin(mnenomic)
            .read()
            .map_err(Error::from)
            .and_then(|out| {
                serde_json::from_str::<Raw>(&out)
                    .map(|raw_key| raw_key.with_backend(backend))
                    .map_err(Error::from)
            })
    }

    /// Initialise the chain state
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    pub fn init_chain(self, moniker: &str, chain_id: &ChainId) -> Result<(), Error> {
        self.0
            .args(["init", moniker, "--chain-id", chain_id.as_str()])
            .run()
            .map_err(Error::from)
    }

    /// Add a genesis account to be given an `amount` of coins.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    pub fn add_genesis_account(self, key: &Key, amount: u128, denom: &str) -> Result<(), Error> {
        self.0
            .args([
                "add-genesis-account",
                key.name(),
                &format!("{amount}{denom}"),
                "--keyring-backend",
                key.backend(),
            ])
            .run()
            .map_err(Error::from)
    }

    /// Add a genesis tx to be made.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    pub fn gentx(
        self,
        key: &Key,
        amount: u128,
        denom: &str,
        gas: u128,
        chain_id: &str,
    ) -> Result<(), Error> {
        self.0
            .args([
                "gentx",
                key.name(),
                &format!("{amount}{denom}"),
                "--gas",
                gas.to_string().as_str(),
                "--chain-id",
                chain_id,
                "--keyring-backend",
                key.backend(),
            ])
            .run()
            .map_err(Error::from)
    }

    /// Collect all the genesis txs
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    pub fn collect_gentx(self) -> Result<(), Error> {
        self.0.arg("collect-gentxs").run().map_err(Error::from)
    }

    /// Validate the genesis file
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    pub fn validate_genesis(self) -> Result<(), Error> {
        self.0.arg("validate-genesis").run().map_err(Error::from)
    }

    /// Build a predictable address
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue with running the command.
    pub fn build_address(
        self,
        code_hash: &str,
        from: &'a Key,
        salt: &str,
    ) -> Result<String, Error> {
        let hex_salt = hex::encode(salt);

        let out = self
            .0
            .args([
                "query",
                "wasm",
                "build-address",
                code_hash,
                from.address(),
                hex_salt.as_str(),
            ])
            .read()?;

        let address = out.split_ascii_whitespace().next().unwrap().to_owned();

        Ok(address)
    }

    #[must_use]
    pub fn tx(self, from: &'a Key, chain_id: &'a ChainId, node: &'a NodeUri) -> BuildTxCmd<'a> {
        BuildTxCmd {
            from,
            chain_id,
            node,
            cmd: self.0,
        }
    }

    #[must_use]
    pub fn query(self, node: &NodeUri) -> QueryCmd<'a> {
        let cmd = self.0.args(["--node", node.as_str()]);
        QueryCmd { cmd }
    }
}

macro_rules! ready {
    ($cmd:ident, $build_tx_cmd:ident) => {{
        let cmd = $cmd.args([
            "--from",
            $build_tx_cmd.from.name(),
            "--keyring-backend",
            $build_tx_cmd.from.backend(),
            "--chain-id",
            $build_tx_cmd.chain_id.as_str(),
            "--node",
            $build_tx_cmd.node.as_str(),
            "--yes",
        ]);

        ReadyTxCmd { cmd }
    }};
}

impl<'a> BuildTxCmd<'a> {
    pub fn wasm_store<P>(self, path: P) -> ReadyTxCmd<'a>
    where
        P: AsRef<Path>,
    {
        let cmd = self.cmd.args(["tx", "wasm", "store"]).arg(path.as_ref());
        ready!(cmd, self)
    }

    #[must_use]
    pub fn wasm_init(
        self,
        code_id: CodeId,
        label: &str,
        msg: &str,
        admin: Option<&str>,
    ) -> ReadyTxCmd<'a> {
        let cmd = self.cmd.args([
            "tx",
            "wasm",
            "instantiate",
            code_id.u64().to_string().as_str(),
            msg,
            "--label",
            label,
        ]);

        let cmd = if let Some(admin) = admin {
            cmd.args(["--admin", admin])
        } else {
            cmd.arg("--no-admin")
        };

        ready!(cmd, self)
    }

    #[must_use]
    pub fn wasm_exec(self, contract: &Contract, msg: &str) -> ReadyTxCmd<'a> {
        let cmd = self
            .cmd
            .args(["tx", "wasm", "execute", contract.as_str(), msg]);
        ready!(cmd, self)
    }
}

#[derive(Deserialize)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct Event {
    pub r#type: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Deserialize)]
pub struct Log {
    pub events: Vec<Event>,
}

#[derive(Deserialize)]
pub struct Hex(String);

#[derive(Clone, PartialEq, Message)]
pub struct MsgData {
    #[prost(string, tag = "1")]
    pub msg_type: String,
    #[prost(bytes, tag = "2")]
    pub data: Vec<u8>,
}

impl MsgData {
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct TxMsgData {
    #[prost(message, repeated, tag = "1")]
    pub data: Vec<MsgData>,
}

#[derive(Display, Clone, Copy, Message)]
pub struct CodeId {
    #[prost(uint64, tag = "1")]
    code_id: u64,
}

impl CodeId {
    #[must_use]
    pub const fn u64(self) -> u64 {
        self.code_id
    }
}

#[derive(Display, Clone, Message)]
pub struct Contract {
    #[prost(string, tag = "1")]
    address: String,
}

impl Contract {
    #[must_use]
    pub fn unchecked(address: String) -> Self {
        Self { address }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.address.as_str()
    }
}

#[derive(Clone, Message)]
pub struct CwExecuteResponse {
    #[prost(bytes, tag = "1")]
    data: Vec<u8>,
}

impl CwExecuteResponse {
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Decode to a `T`
    ///
    /// # Errors
    ///
    /// This function will return an error if JSON deserialization fails.
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, Error> {
        serde_json::from_slice(self.as_slice()).map_err(Error::from)
    }

    /// Decode into a `T`
    ///
    /// # Errors
    ///
    pub fn decode_into<T: DeserializeOwned>(self) -> Result<T, Error> {
        self.decode()
    }
}

#[derive(Deserialize)]
pub struct Metadata {
    pub txhash: String,
    pub code: u32,
    pub raw_log: String,
    pub logs: Vec<Log>,
}

#[derive(Deserialize)]
pub struct TxData<D> {
    #[serde(flatten)]
    pub meta: Metadata,
    pub data: D,
}

pub type RawTxData = TxData<Hex>;

impl<Data> TxData<Data> {
    pub fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.meta
            .logs
            .iter()
            .flat_map(|l| l.events.as_slice())
            .flat_map(|ev| ev.attributes.as_slice())
    }

    pub fn into_data(self) -> Data {
        self.data
    }
}

impl RawTxData {
    /// Decode the raw data hex string into the `Msg` type
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Hex decoding fails
    /// - There is not at least one `MsgData` in the reply
    /// - Protobuf decoding fails
    pub fn decode<Msg>(self) -> Result<TxData<Msg>, Error>
    where
        Msg: Message + Default,
    {
        let TxData { meta, data } = self;

        let bytes = hex::decode(data.0)?;

        TxMsgData::decode(bytes.as_slice())?
            .data
            .first()
            .ok_or(Error::ExpectedAtLeastOneMsgData)
            .map(MsgData::as_slice)
            .and_then(|data| Msg::decode(data).map_err(Error::from))
            .map(|data| TxData { meta, data })
    }
}

impl<'a> ReadyTxCmd<'a> {
    #[must_use]
    pub fn amount(self, amount: u128, denom: &str) -> Self {
        let cmd = self.cmd.args(["--amount", &format!("{amount}{denom}")]);
        Self { cmd }
    }

    /// Execute the `TxCmd`, returning the tx ID for querying
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue running the command
    /// - JSON Deserialisation fails
    pub fn execute(self, gas: &Gas) -> Result<TxId, Error> {
        let cmd = self.cmd.args([
            "--gas",
            gas.units.to_string().as_str(),
            "--gas-prices",
            gas.price.to_string().as_str(),
            "--output",
            "json",
        ]);

        println!("{cmd}");

        let tx_exec_str = cmd.read()?;

        let tx_exec: RawTxData = serde_json::from_str(&tx_exec_str)?;

        if tx_exec.meta.code > 0 {
            return Err(Error::TxExecute(tx_exec.meta.raw_log));
        }

        Ok(TxId::from(tx_exec.meta.txhash))
    }
}

#[derive(Debug, Display, Deserialize, FromStr, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockHeight(u64);

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct SyncInfo {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub latest_block_height: BlockHeight,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Status {
    #[serde(rename = "SyncInfo")]
    pub sync_info: SyncInfo,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CodeInfo {
    pub creator: String,
    pub data_hash: String,
}

impl<'a> QueryCmd<'a> {
    /// Query the tx ID returning `None` if it cannot yet be found.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue running the command
    /// - The response contains an error
    /// - Parsing UTF-8 fails from stderr fails
    /// - JSON deserialisation fails
    pub fn tx(self, tx_id: &TxId) -> Result<Option<RawTxData>, Error> {
        let output = self
            .cmd
            .args(["query", "tx", tx_id.as_str(), "--output", "json"])
            .ignore_status()
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)?;

            if stderr.contains("not found") {
                return Ok(None);
            }

            return Err(Error::TxExecute(stderr));
        }

        let tx_data: RawTxData = serde_json::from_slice(&output.stdout)?;

        if tx_data.meta.code > 0 {
            return Err(Error::TxExecute(tx_data.meta.raw_log));
        }

        Ok(Some(tx_data))
    }

    /// Query the node status returning `None` if it cannot yet be found.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue running the command
    /// - The response contains an error
    /// - Parsing UTF-8 fails from stderr fails
    /// - JSON deserialisation fails
    pub fn status(self) -> Result<Option<Status>, Error> {
        let output = self.cmd.arg("status").ignore_status().output()?;

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)?;

            if stderr.contains("connection refused") {
                return Ok(None);
            }

            return Err(Error::TxExecute(stderr));
        }

        serde_json::from_slice(&output.stdout)
            .map(Some)
            .map_err(Error::from)
    }

    /// Query the `contract` with the query `msg`
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue running the command
    pub fn wasm_smart(self, contract: &Contract, msg: &str) -> Result<String, Error> {
        self.cmd
            .args([
                "query",
                "wasm",
                "contract-state",
                "smart",
                contract.as_str(),
                msg,
                "--output",
                "json",
            ])
            .read()
            .map_err(Error::from)
    }

    /// Query the code info for the stored `code_id`
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an issue running the command
    pub fn code_info(self, code_id: CodeId) -> Result<CodeInfo, Error> {
        self.cmd
            .args([
                "query",
                "wasm",
                "code-info",
                code_id.to_string().as_str(),
                "--output",
                "json",
            ])
            .read()
            .map_err(Error::from)
            .and_then(|json| serde_json::from_str(&json).map_err(Error::from))
    }
}

/// Keep querying the tx ID until it is found
///
/// # Errors
///
/// This function will return an error if `QueryCmd::tx` returns an error.
pub fn wait_for_tx(sh: &Shell, network: &dyn Network, tx_id: &TxId) -> Result<RawTxData, Error> {
    let node_uri = network.node_uri(sh)?;

    loop {
        if let Some(tx_data) = network.cli(sh)?.query(&node_uri).tx(tx_id)? {
            return Ok(tx_data);
        }

        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// Keep querying the network for block height until it is found
///
/// # Errors
///
/// This function will return an error if `QueryCmd::tx` returns an error.
#[allow(clippy::missing_panics_doc)]
pub fn wait_for_blocks(sh: &Shell, network: &dyn Network) -> Result<BlockHeight, Error> {
    let node_uri = network.node_uri(sh)?;

    loop {
        if let Some(status) = network.cli(sh)?.query(&node_uri).status()? {
            let start_height = status.sync_info.latest_block_height;

            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));

                let status = network
                    .cli(sh)?
                    .query(&node_uri)
                    .status()?
                    .expect("status already found once");

                let current_height = status.sync_info.latest_block_height;

                if current_height > start_height {
                    return Ok(status.sync_info.latest_block_height);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}
