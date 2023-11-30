#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Shell(#[from] xshell::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    CtrlC(#[from] ctrlc::Error),
    #[error(transparent)]
    Bip39(#[from] bip39::Error),
    #[error(transparent)]
    ParseUtf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseHex(#[from] hex::FromHexError),
    #[error(transparent)]
    ParseProtobuf(#[from] prost::DecodeError),
    #[error(transparent)]
    StdIo(#[from] std::io::Error),
    #[error("{0}")]
    CmdExecute(String),
    #[error("{0}")]
    TxExecute(String),
    #[error("expected code id")]
    ExpectedCodeId,
    #[error("expected at least one message response in tx data")]
    ExpectedAtLeastOneMsgResponse,
}

pub mod cli;
pub mod contract;
pub mod key;
pub mod network;
pub mod ops;

pub use cli::wait_for_blocks;
pub use contract::{execute, instantiate, query, store};
pub use network::{
    archway::{CmdExt as ArchwayCmdExt, Local as ArchwayLocalnet},
    gas::Prices as GasPrices,
    neutron::local::Local as NeutronLocalnet,
    neutron::testnet::Testnet as NeutronTestnet,
    Initialize, IntoForeground, Keys, Network, StartLocal,
};
