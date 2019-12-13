#!/bin/zsh

cargo test longhauling_task_join -- --ignored --exact --nocapture
cargo test slow_join_interrupted -- --ignored --exact --nocapture
cargo test slow_join -- --ignored --exact --nocapture