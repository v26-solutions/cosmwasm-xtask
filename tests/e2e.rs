use anyhow::Result;
use xshell::Shell;

use cosmwasm_xtask::{
    cli::wait_for_blocks, execute, instantiate, query, store, ArchwayLocalnet, Initialize,
    StartLocal,
};

#[test]
fn archway_localnet() -> Result<()> {
    let sh = Shell::new()?;

    let network = ArchwayLocalnet::initialize(&sh)?;

    let _handle = network.start_local(&sh)?;

    let _block = wait_for_blocks(&sh, &network)?;

    let demo_account = network.keys.first().expect("at least one account");

    let code_id = store(&sh, &network, demo_account, "examples/cw20_base.wasm", None)?;

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

    assert_eq!(balance.balance.u128(), 1_000_000_000_000u128);

    Ok(())
}
