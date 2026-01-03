run-preview:
    cargo run --manifest-path preview/Cargo.toml

publish:
    cd lib; cargo publish

format *ARGS:
    cargo +nightly fmt {{ARGS}}

lint *ARGS:
    cargo clippy {{ARGS}}

lint-fix *ARGS:
    just lint --fix --allow-dirty {{ARGS}}