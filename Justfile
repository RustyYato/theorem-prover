list:
    just --list
test:
    cargo test --lib
    cargo test --doc
    cargo test --test kind_cmp -r -- --nocapture
    cargo test --test level_cmp -r -- --nocapture
    just test-univ
test-univ:
    cargo test --test univ_cmp -r -- --nocapture
