#!/usr/bin/env bash
# Builds the browser checker (reqwasm) and writes reqwasm.js + reqwasm_bg.wasm to $1
# (default: target/wasm-out). The documentation site commits these two files, see
# `make playground` in the lang-doc repository.
set -euo pipefail

cd "$(dirname "$0")/.."
out="${1:-target/wasm-out}"

for tool in cargo rustup wasm-bindgen; do
    command -v "$tool" >/dev/null || {
        echo "missing required tool: $tool" >&2
        exit 1
    }
done

# wasm-bindgen-cli must be the version of the wasm-bindgen crate in Cargo.lock
locked=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/[^0-9.]/, "", $0); print; exit }' Cargo.lock)
installed=$(wasm-bindgen --version | awk '{ print $2 }')
if [[ "$locked" != "$installed" ]]; then
    echo "wasm-bindgen-cli is $installed but Cargo.lock has $locked: cargo install wasm-bindgen-cli --version $locked --locked" >&2
    exit 1
fi

rustup target add wasm32-unknown-unknown
cargo build --profile wasm --target wasm32-unknown-unknown -p reqwasm

mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" \
    target/wasm32-unknown-unknown/wasm/reqwasm.wasm

ls -l "$out"/reqwasm.js "$out"/reqwasm_bg.wasm
