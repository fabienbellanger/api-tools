# API Tools

[![Build status](https://github.com/fabienbellanger/api-tools/actions/workflows/CI.yml/badge.svg?branch=main)](https://github.com/fabienbellanger/api-tools/actions/workflows/CI.yml)
[![Crates.io](https://img.shields.io/crates/v/api-tools)](https://crates.io/crates/api-tools)
[![Documentation](https://docs.rs/api-tools/badge.svg)](https://docs.rs/api-tools)

> Toolkit for API in Rust

## Installation

For standard functionalities, no additional dependencies are required:

```toml
[dependencies]
api-tools = "*"
```

If you need all [features](#Features-list), you can use the `full` feature:

```toml
[dependencies]
api-tools = { version = "*", features = ["full"] }
```

Or you can use `cargo add` command:

```bash
cargo add api-tools
cargo add api-tools -F full
```

## Features list

| Name   | Description         | Default |
| ------ | ------------------- | :-----: |
| `axum` | Enable Axum feature |   ❌    |
| `full` | Enable all features |   ❌    |

## Examples

TODO

## Code coverage

Tool used: [tarpaulin](https://github.com/xd009642/tarpaulin)

```shell
cargo install cargo-tarpaulin
```

```shell
cargo tarpaulin --all-features --ignore-tests --line --count --include-files src/**/*
```

To generation HTML file [`tarpaulin-report.html`](tarpaulin-report.html):

```shell
cargo tarpaulin --all-features --ignore-tests --line --count --include-files src/**/* --out Html
```

_Results:_

- [2025-05-20] `55.37% coverage, 170/307 lines covered`

## MSRV

Tool used: [cargo-msrv](https://github.com/foresterre/cargo-msrv)

```shell
cargo install cargo-msrv
```

```shell
cargo msrv find
cargo msrv verify
```
