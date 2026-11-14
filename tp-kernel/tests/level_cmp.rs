use std::sync::atomic::AtomicU64;

use rand::RngExt;
use tp_kernel::{Kind, Level};

#[test]
fn test_mt() {
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().unwrap().get() {
            s.spawn(main);
        }
    });
}

// #[test]
fn main() {
    debug_assert!(false, "This test should be run in release mode");

    static C: AtomicU64 = AtomicU64::new(0);

    let mut i = 0;
    let f = 1_000_000 + rand::random_range(200_000..1_000_000);
    // for _ in 0..100_000_000 {
    loop {
        i += 1;
        if i % f == 0 {
            let c = i + C.fetch_add(i, std::sync::atomic::Ordering::Relaxed);
            i = 0;
            println!("{}", c / 1_000_000);
            if c >= 1_000_000_000 {
                return;
            }
        }
        let a = rand_level(&mut rand::rng());

        // test that le is reflexive
        assert!(a.le(&a), "REFL {a:?}");
        let b = rand_level(&mut rand::rng());
        let c = rand_level(&mut rand::rng());
        if a.le(&b) {
            if b.le(&c) {
                // test that le is transitive
                assert!(a.le(&c), "TRANS {a:?} <= {c:?} || {b:?}")
            }

            // test that trunc is monotone
            assert!(
                a.clone().succ().le(&b.clone().succ()),
                "SUCC {a:?} <= {b:?}"
            );
            let k = rand_kind(&mut rand::rng());
            assert!(
                a.clone().when(k.clone()).le(&b.clone().when(k.clone())),
                "{a:?} <= {b:?} [{k:?}]"
            );
        }

        // eq is the antisymmetrization of le, which is an equivalence relation when le is a preorder (reflexive and transitive)
        assert_eq!(a == b, a <= b && b <= a, "{a:?} == {b:?}");

        // partial_cmp is correct
        assert_eq!(
            a.partial_cmp(&b),
            match (a.le(&b), b.le(&a)) {
                (false, false) => None,
                (true, false) => Some(std::cmp::Ordering::Less),
                (false, true) => Some(std::cmp::Ordering::Greater),
                (true, true) => Some(std::cmp::Ordering::Equal),
            }
        )
    }
}

fn rand_level(rng: &mut dyn rand::Rng) -> Level {
    match rng.random_range(0..5) {
        0 => Level::Zero,
        1 => Level::Param(rng.random_range(0..20)),
        2 => rand_level(rng).succ(),
        3 => rand_level(rng).when(rand_simple_kind(rng)),
        4 => rand_level(rng).max(rand_level(rng)),
        _ => unreachable!(),
    }
}

fn rand_simple_kind(rng: &mut dyn rand::Rng) -> Kind {
    match rng.random_range(0..3) {
        0 => Kind::SProp,
        1 => Kind::Prop,
        2 => Kind::Type,
        _ => unreachable!(),
    }
}

fn rand_kind(rng: &mut dyn rand::Rng) -> Kind {
    match rng.random_range(0..6) {
        0 => Kind::SProp,
        1 => Kind::Prop,
        2 => Kind::Type,
        3 => Kind::Param(rng.random_range(0..20)),
        4 => rand_kind(rng).trunc(),
        5 => rand_kind(rng).max(rand_kind(rng)),
        _ => unreachable!(),
    }
}
