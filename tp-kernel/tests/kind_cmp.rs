use std::sync::atomic::AtomicU64;

use rand::RngExt;
use tp_kernel::Sort;

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
        let a = rand_kind(&mut rand::rng());

        // test that le is reflexive
        assert!(a.le(&a));
        let b = rand_kind(&mut rand::rng());
        let c = rand_kind(&mut rand::rng());
        if a.le(&b) {
            if b.le(&c) {
                // test that le is transitive
                assert!(a.le(&c), "{a:?} <= {b:?} <= {c:?}")
            }

            // test that trunc is monotone
            assert!(a.clone().trunc().le(&b.clone().trunc()), "{a:?} <= {b:?}");
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

fn rand_kind(rng: &mut dyn rand::Rng) -> Sort {
    match rng.random_range(0..6) {
        0 => Sort::SProp,
        1 => Sort::Prop,
        2 => Sort::Type,
        3 => Sort::Param(rng.random_range(0..20)),
        4 => Sort::Trunc(rand_kind(rng).into()),
        5 => Sort::Max(rand_kind(rng).into(), rand_kind(rng).into()),
        _ => unreachable!(),
    }
}
