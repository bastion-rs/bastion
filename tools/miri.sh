rustup component add miri
cargo miri setup
cargo clean
# Do some Bastion shake
pushd src/bastion && \
    MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks" cargo miri test --features lever/nightly dispatcher && \
    MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks" cargo miri test --features lever/nightly path && \
    MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks" cargo miri test --features lever/nightly broadcast && \
    MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks" cargo miri test --features lever/nightly children_ref && \
popd
