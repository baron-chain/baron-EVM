#!/bin/bash

# stop on error
set -e

cargo publish --package bcevm-primitives
cargo publish --package bcevm-precompile
cargo publish --package bcevm-interpreter
cargo publish --package bcevm
cargo publish --package bcevme

echo "All crates published"
