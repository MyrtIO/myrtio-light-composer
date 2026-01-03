run-preview:
    cargo run --manifest-path preview/Cargo.toml

publish:
    cargo publish

format *ARGS:
    cargo +nightly fmt {{ARGS}}

lint *ARGS:
    cargo clippy {{ARGS}}

lint-fix *ARGS:
    just lint --fix --allow-dirty {{ARGS}}

test *ARGS:
    cargo test {{ARGS}}