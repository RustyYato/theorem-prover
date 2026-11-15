use std::sync::atomic::AtomicU64;

use rand::RngExt;
use tp_kernel::{Kind, Level, Universe};

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
        if i > 0 && i % f == 0 {
            let c = i + C.fetch_add(i, std::sync::atomic::Ordering::Relaxed);
            i = 0;
            println!("{}", c / 1_000_000);
            // if c >= 1_000_000_000 {
            //     return;
            // }
        }
        // if i % 1_000_000 == 0 {
        //     let i = i / 1_000_000;
        //     println!("{i}")
        // }
        let a = rand_univ(&mut rand::rng());

        // test that le is reflexive
        assert!(a.le(&a), "REFL {a:?}");
        let b = rand_univ(&mut rand::rng());
        let c = rand_univ(&mut rand::rng());
        if a.le(&b) && b.le(&c) {
            i += 1;
            // test that le is transitive
            assert!(a.le(&c), "TRANS {a:?} <= {c:?} || {b:?}")
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

fn rand_univ(rng: &mut dyn rand::Rng) -> Universe {
    Universe {
        kind: rand_kind(rng),
        level: rand_level(rng),
    }
}

fn rand_level(rng: &mut dyn rand::Rng) -> Level {
    match rng.random_range(0..5) {
        0 => Level::Zero,
        1 => Level::Param(rng.random_range(0..20)),
        2 => Level::Succ(rand_level(rng).into()),
        3 => Level::When(rand_simple_kind(rng).into(), rand_level(rng).into()),
        4 => Level::Max(rand_level(rng).into(), rand_level(rng).into()),
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
        4 => Kind::Trunc(rand_kind(rng).into()),
        5 => Kind::Max(rand_kind(rng).into(), rand_kind(rng).into()),
        _ => unreachable!(),
    }
}
