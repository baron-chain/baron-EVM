#!/bin/bash
set -e

packages=(
    "bcevm-primitives"
    "bcevm-precompile"
    "bcevm-interpreter"
    "bcevm"
    "bcevme"
)

for package in "${packages[@]}"; do
    cargo publish --package "$package"
done

echo "All crates published successfully"
