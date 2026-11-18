#!/usr/bin/env bash
set -euo pipefail

TARGETS=(
    x86_64-unknown-linux-musl
    aarch64-unknown-linux-musl
    x86_64-apple-darwin
    aarch64-apple-darwin
    x86_64-pc-windows-gnu
)
BINARIES=(reqtool reqlsp)

cd "$(dirname "$0")/.."

for tool in cargo cargo-zigbuild zig jq rustup; do
    command -v "$tool" >/dev/null || {
        echo "missing required tool: $tool" >&2
        exit 1
    }
done

metadata=$(cargo metadata --no-deps --format-version 1)
version_of() {
    jq -r --arg n "$1" '.packages[] | select(.name == $n) | .version' <<<"$metadata"
}

mkdir -p release
failed=()

for target in "${TARGETS[@]}"; do
    echo "==> $target"
    ext=""
    [[ "$target" == *windows* ]] && ext=".exe"

    if ! rustup target add "$target" ||
        ! cargo zigbuild --release --target "$target" \
            $(printf -- '-p %s ' "${BINARIES[@]}"); then
        failed+=("$target")
        continue
    fi

    for bin in "${BINARIES[@]}"; do
        out="release/${bin}_$(version_of "$bin")_${target}${ext}"
        cp "target/${target}/release/${bin}${ext}" "$out"
        echo "    $out"
    done
done

if ((${#failed[@]})); then
    echo "failed targets: ${failed[*]}" >&2
    exit 1
fi
