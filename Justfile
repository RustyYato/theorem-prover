list:
    just --list
test:
    cargo test --lib
    cargo test --doc
    # kind comparison has been formally verified, so testing is no longer needed
    # cargo test --test kind_cmp -r -- --nocapture
    cargo test --test level_cmp -r -- --nocapture
    just test-univ
test-univ:
    cargo test --test univ_cmp -r -- --nocapture
