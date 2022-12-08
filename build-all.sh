#!/usr/bin/env bash

set -eu

cargo +stable contract build --manifest-path erc20/Cargo.toml
cargo +stable contract build