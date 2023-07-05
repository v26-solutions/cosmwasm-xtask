use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use xshell::Shell;

use cosmwasm_xtask::{
    contract::{execute, instantiate, query, store},
    network::{archway::Local as ArchwayLocal, Clean, Initialize, IntoForeground, StartLocal},
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "start local network")]
    StartLocal,
    #[command(about = "clean network state")]
    Clean,
    #[command(about = "deploy contract to the network")]
    Deploy,
}

pub fn main() -> Result<()> {
    let cli = Cli::parse();

    let sh = Shell::new()?;

    match cli.command {
        Command::StartLocal => ArchwayLocal::initialize(&sh)?
            .start_local(&sh)?
            .into_foreground()?,

        Command::Clean => ArchwayLocal::initialize(&sh)?.clean(&sh)?,

        Command::Deploy => {
            let network = ArchwayLocal::initialize(&sh)?;

            let demo_account = network
                .keys
                .first()
                .ok_or_else(|| anyhow!("No demo account"))?;

            let code_id = store(&sh, &network, demo_account, "examples/cw20_base.wasm", None)?;

            println!("Stored CW20 base at code id: {code_id}");

            let contract = instantiate(
                &sh,
                &network,
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
                &sh,
                &network,
                &contract,
                demo_account,
                &cw20::Cw20ExecuteMsg::Mint {
                    recipient: demo_account.address().to_owned(),
                    amount: 1_000_000_000_000u128.into(),
                },
                None,
            )?;

            let balance: cw20::BalanceResponse = query(
                &sh,
                &network,
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
        }
    }

    Ok(())
}
