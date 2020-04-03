#!/usr/bin/env bash
set -eo pipefail

rustup update
rustup toolchain install nightly

if [[ "$OSTYPE" == "darwin"* ]]; then
  TARGET_SELECT=''
else
  TARGET_SELECT='x86_64-unknown-linux-gnu'
fi

# Run SANs
echo "LSAN"
cargo clean
export RUSTFLAGS="-Z sanitizer=leak"
cargo +nightly build --examples --target ${TARGET_SELECT}
sudo -E target/${TARGET_SELECT}/debug/examples/parallel_computation
sudo -E target/${TARGET_SELECT}/debug/examples/callbacks

# -------------------------------

echo "TSAN"
cargo clean
export RUSTFLAGS="-Z sanitizer=thread"
export TSAN_OPTIONS=suppressions=../../../../tools/tsan.suppressions
cargo +nightly build --examples --target ${TARGET_SELECT}
sudo -E target/${TARGET_SELECT}/debug/examples/parallel_computation
sudo -E target/${TARGET_SELECT}/debug/examples/send_recv
sudo -E target/${TARGET_SELECT}/debug/examples/callbacks
