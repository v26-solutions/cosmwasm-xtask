# CosmWasm 🤝 xtask

An [`xshell`-based](https://github.com/matklad/xshell) utility crate for scripting with CosmWasm contracts

Includes: 

- A set of traits for defining different CosmWasm networks at different scopes, e.g. Localnet, Testnet, and even Mainnet.

- A set of functions to `store`, `instantiate`, `execute` and `query` contracts on any given CosmWasm network.

Check `examples/cli.rs` for an example of how to create an [`xtask`-style tool](https://github.com/matklad/cargo-xtask)

Check `tests/e2e.rs` for an example of to do E2E contract tests against live nodes using Cargo's built-in test runner.

## Try it out

```
❯ : cargo r --example cli --
Usage: cli <NETWORK> <COMMAND>

Commands:
  start-local  start local network
  clean        clean network state
  deploy       deploy contract to the network
  help         Print this message or the help of the given subcommand(s)

Arguments:
  <NETWORK>  [possible values: archway-local, neutron-local]

Options:
  -h, --help     Print help
  -V, --version  Print version

```

## Contribute

PRs are very welcome to add more networks, functions and common tasks!
