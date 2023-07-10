#![deny(clippy::all)]
#![warn(clippy::pedantic)]

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
    #[error("{0}")]
    TxExecute(String),
    #[error("expected code id")]
    ExpectedCodeId,
    #[error("expected at least one message data in response")]
    ExpectedAtLeastOneMsgData,
}

pub mod cli;
pub mod contract;
pub mod key;
pub mod network;

pub use cli::wait_for_blocks;
pub use contract::{execute, instantiate, query, store};
pub use network::{
    archway::Local as ArchwayLocalnet, gas::Prices as GasPrices, neutron::Local as NeutronLocalnet,
    neutron::Testnet as NeutronTestnet, Initialize, IntoForeground, Keys, Network, StartLocal,
};
