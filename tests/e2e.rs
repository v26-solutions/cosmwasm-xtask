use anyhow::Result;
use serial_test::serial;
use xshell::Shell;

use cosmwasm_xtask::{
    cli::wait_for_blocks, execute, instantiate, query, store, ArchwayLocalnet, Initialize, Network,
    NeutronLocalnet, StartLocal,
};

fn deploy(sh: &Shell, network: &dyn Network) -> Result<()> {
    let demo_account = network.keys().first().expect("at least one account");

    wait_for_blocks(sh, network)?;

    let code_id = store("examples/cw20_base.wasm").send(sh, network, demo_account)?;

    let contract = instantiate(
        code_id,
        "demo_cw20",
        cw20_base::msg::InstantiateMsg {
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
    )
    .send(sh, network, demo_account)?;

    execute(
        &contract,
        cw20::Cw20ExecuteMsg::Mint {
            recipient: demo_account.address().to_owned(),
            amount: 1_000_000_000_000u128.into(),
        },
    )
    .send(sh, network, demo_account)?;

    let balance: cw20::BalanceResponse = query(
        sh,
        network,
        &contract,
        &cw20::Cw20QueryMsg::Balance {
            address: demo_account.address().to_owned(),
        },
    )?;

    assert_eq!(balance.balance.u128(), 1_000_000_000_000u128);

    Ok(())
}

#[test]
#[serial]
fn archway_localnet() -> Result<()> {
    let sh = Shell::new()?;

    let network = ArchwayLocalnet::initialize(&sh)?;

    let _handle = network.start_local(&sh)?;

    deploy(&sh, &network)
}

#[test]
#[serial]
fn neutron_localnet() -> Result<()> {
    let sh = Shell::new()?;

    let network = NeutronLocalnet::initialize(&sh)?;

    let _handle = network.start_local(&sh)?;

    deploy(&sh, &network)
}
