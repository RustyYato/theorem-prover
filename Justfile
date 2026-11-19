list:
    just --list
test:
    cargo test --lib
    cargo test --doc
    # kind comparison has been formally verified, so testing is no longer needed
    # cargo test --test kind_cmp -r -- --nocapture
    RUST_FLAGS='--cfg tp_disable_normalize' cargo test --target-dir target/tp_disable_normalize --test level_cmp -r -- --nocapture
    just test-univ
test-univ:
    RUST_FLAGS='--cfg tp_disable_normalize' cargo test --target-dir target/tp_disable_normalize --test univ_cmp -r -- --nocapture
