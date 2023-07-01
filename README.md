# CosmWasm 🤝 xtask

An [`xshell`-based](https://github.com/matklad/xshell) utility crate for scripting with CosmWasm contracts

Includes: 

- A set of traits for defining different CosmWasm networks at different scopes, e.g. Localnet, Testnet, and even Mainnet.

- A set of functions to `store`, `instantiate`, `execute` and `query` contracts on any given CosmWasm network.

Check `examples/cli.rs` for an example of how to create an [`xtask`-style tool](https://github.com/matklad/cargo-xtask)

## Try it out

```
$ cargo r --example cli -- start-local

// In another shell
$ cargo r --example cli -- deploy
```

## Contribute

PRs are very welcome to add more networks, functions and common tasks!
