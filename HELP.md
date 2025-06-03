# HELP

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
