use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use xshell::Shell;

use cosmwasm_xtask::{
    contract::{execute, instantiate, query, store},
    key::KeyringBackend,
    network::{Clean, Network},
    ArchwayLocalnet, Initialize, IntoForeground, Keys, NeutronLocalnet, NeutronTestnet, StartLocal,
};

#[derive(ValueEnum, Clone, Copy)]
enum NetworkOption {
    ArchwayLocal,
    NeutronLocal,
    NeutronTestnet,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    network: NetworkOption,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "start local network")]
    StartLocal,
    #[command(about = "clean network state")]
    Clean,
    #[command(about = "deploy contract to the network")]
    Deploy,
    #[command(about = "list the keys")]
    Keys,
}

/// Deploy on any network
pub fn deploy(sh: &Shell, network: &dyn Network) -> Result<()> {
    let demo_account = network
        .keys()
        .first()
        .ok_or_else(|| anyhow!("No demo account"))?;

    let code_id = store(sh, network, demo_account, "examples/cw20_base.wasm", None)?;

    println!("Stored CW20 base at code id: {code_id}");

    let contract = instantiate(
        sh,
        network,
        code_id,
        demo_account,
        "demo_cw20",
        &cw20_base::msg::InstantiateMsg {
            name: "Demo".into(),
            symbol: "DEMO".into(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(cw20::MinterResponse {
                minter: demo_account.address().to_owned(),
                cap: None,
            }),
            marketing: None,
        },
        None,
    )?;

    println!("Instantiated CW20 DEMO at address: {contract}");

    println!("Minting 1,000,000 DEMO to {}", demo_account.address());

    execute(
        sh,
        network,
        &contract,
        demo_account,
        &cw20::Cw20ExecuteMsg::Mint {
            recipient: demo_account.address().to_owned(),
            amount: 1_000_000_000_000u128.into(),
        },
        None,
    )?;

    let balance: cw20::BalanceResponse = query(
        sh,
        network,
        &contract,
        &cw20::Cw20QueryMsg::Balance {
            address: demo_account.address().to_owned(),
        },
    )?;

    println!(
        "Balance of {}: {} uDEMO",
        demo_account.address(),
        balance.balance
    );

    Ok(())
}

pub fn main() -> Result<()> {
    let cli = Cli::parse();

    let sh = Shell::new()?;

    match cli.command {
        Command::StartLocal => match cli.network {
            NetworkOption::ArchwayLocal => ArchwayLocalnet::initialize(&sh)?
                .start_local(&sh)?
                .into_foreground()?,

            NetworkOption::NeutronLocal => NeutronLocalnet::initialize(&sh)?
                .start_local(&sh)?
                .into_foreground()?,

            _ => bail!("only localnets can be started"),
        },

        Command::Clean => match cli.network {
            NetworkOption::ArchwayLocal => ArchwayLocalnet::initialize(&sh)?.clean(&sh)?,
            NetworkOption::NeutronLocal => NeutronLocalnet::initialize(&sh)?.clean(&sh)?,
            NetworkOption::NeutronTestnet => NeutronTestnet::initialize(&sh)?.clean(&sh)?,
        },

        Command::Deploy => match cli.network {
            NetworkOption::ArchwayLocal => ArchwayLocalnet::initialize(&sh)
                .map_err(anyhow::Error::from)
                .and_then(|network| deploy(&sh, &network))?,

            NetworkOption::NeutronLocal => NeutronLocalnet::initialize(&sh)
                .map_err(anyhow::Error::from)
                .and_then(|network| deploy(&sh, &network))?,

            NetworkOption::NeutronTestnet => {
                let mut network = NeutronTestnet::initialize(&sh)?;

                if network.keys.is_empty() {
                    network.recover(
                        &sh,
                        "demo",
                        cosmwasm_xtask::network::neutron::DEMO_MNEMONIC_3,
                        KeyringBackend::Test,
                    )?;
                }

                deploy(&sh, &network)?
            }
        },

        Command::Keys => match cli.network {
            NetworkOption::ArchwayLocal => ArchwayLocalnet::initialize(&sh)?.keys().to_owned(),
            NetworkOption::NeutronLocal => NeutronLocalnet::initialize(&sh)?.keys().to_owned(),
            NetworkOption::NeutronTestnet => NeutronTestnet::initialize(&sh)?.keys().to_owned(),
        }
        .into_iter()
        .for_each(|key| println!("{key}")),
    }

    Ok(())
}
