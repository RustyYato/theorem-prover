use std::cmp::Ordering;

use triomphe::Arc;

use crate::UniverseParams;

const ENABLE_NORMALIZATION: bool = !cfg!(tp_disable_normalize);

#[derive(Clone)]
pub struct Sort {
    kind: SortKind,
}

impl core::fmt::Debug for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Debug, Clone)]
enum SortKind {
    SProp,
    Prop,
    Type,
    Param(u16),
    Trunc(Arc<Sort>),
    Max(Arc<Sort>, Arc<Sort>),
}

#[derive(Clone)]
pub struct Level {
    minv: i32,
    maxv: i32,
    kind: LevelKind,
}

impl core::fmt::Debug for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Debug, Clone)]
enum LevelKind {
    Zero,
    Param(u16),
    Add(Arc<Level>, u16),
    When(Arc<Sort>, Arc<Level>),
    Max(Arc<Level>, Arc<Level>),
}

#[derive(Debug, Clone)]
pub struct Universe {
    pub sort: Sort,
    pub level: Level,
}

impl Eq for Sort {}
impl PartialEq for Sort {
    fn eq(&self, other: &Self) -> bool {
        if self.fast_eq(other) {
            return true;
        }

        self <= other && other <= self
    }
}

impl PartialOrd for Sort {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // handle all combinations of sprop, prop, type, and param
        // since they are easy and cheap to check, then fall back to
        // general alg
        match (&self.kind, &other.kind) {
            (SortKind::SProp, SortKind::SProp)
            | (SortKind::Prop, SortKind::Prop)
            | (SortKind::Type, SortKind::Type) => {
                return Some(std::cmp::Ordering::Equal);
            }
            (SortKind::Param(i), SortKind::Param(j)) => {
                return if i == j { Some(Ordering::Equal) } else { None };
            }
            (SortKind::Param(_), SortKind::Prop) | (SortKind::Prop, SortKind::Param(_)) => {
                return None;
            }
            (SortKind::SProp, SortKind::Param(_))
            | (SortKind::Param(_), SortKind::Type)
            | (SortKind::SProp, SortKind::Prop)
            | (SortKind::SProp, SortKind::Type)
            | (SortKind::Prop, SortKind::Type) => {
                return Some(Ordering::Less);
            }
            (SortKind::Param(_), SortKind::SProp)
            | (SortKind::Type, SortKind::Param(_))
            | (SortKind::Prop, SortKind::SProp)
            | (SortKind::Type, SortKind::SProp)
            | (SortKind::Type, SortKind::Prop) => {
                return Some(Ordering::Greater);
            }
            (SortKind::Max(_, _), _)
            | (_, SortKind::Max(_, _))
            | (SortKind::Trunc(_), _)
            | (_, SortKind::Trunc(_)) => (),
        }

        match (self.le(other), other.le(self)) {
            (false, false) => None,
            (true, false) => Some(std::cmp::Ordering::Less),
            (false, true) => Some(std::cmp::Ordering::Greater),
            (true, true) => Some(std::cmp::Ordering::Equal),
        }
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        self.le(other)
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        other.le(self)
    }
}

impl Eq for Level {}
impl PartialEq for Level {
    fn eq(&self, other: &Self) -> bool {
        if self.fast_eq(other) {
            return true;
        }

        self <= other && other <= self
    }
}

impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.le(other), other.le(self)) {
            (false, false) => None,
            (true, false) => Some(std::cmp::Ordering::Less),
            (false, true) => Some(std::cmp::Ordering::Greater),
            (true, true) => Some(std::cmp::Ordering::Equal),
        }
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        self.le(other)
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        other.le(self)
    }
}

impl Eq for Universe {}
impl PartialEq for Universe {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.sort.le_prop(), other.sort.le_prop()) {
            (true, true) => return self.sort == other.sort,
            (true, false) | (false, true) => return false,
            (false, false) => (),
        }

        if self.sort.fast_eq(&other.sort) && self.level.fast_eq(&other.level) {
            return true;
        }

        self <= other && other <= self
    }
}
impl PartialOrd for Universe {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.le(other), other.le(self)) {
            (false, false) => None,
            (true, false) => Some(std::cmp::Ordering::Less),
            (false, true) => Some(std::cmp::Ordering::Greater),
            (true, true) => Some(std::cmp::Ordering::Equal),
        }
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        self.le(other)
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        other.le(self)
    }
}

impl Sort {
    pub const fn sprop() -> Sort {
        Self {
            kind: SortKind::SProp,
        }
    }

    pub const fn prop() -> Sort {
        Self {
            kind: SortKind::Prop,
        }
    }

    pub const fn ty() -> Sort {
        Self {
            kind: SortKind::Type,
        }
    }

    pub const fn param(i: u16) -> Sort {
        Self {
            kind: SortKind::Param(i),
        }
    }

    pub fn trunc(self) -> Self {
        if ENABLE_NORMALIZATION {
            match self.kind {
                SortKind::Trunc(_) => return self,
                SortKind::SProp => return Sort::sprop(),
                SortKind::Prop | SortKind::Type => return Sort::prop(),
                SortKind::Max(_, _) => {
                    if self <= Sort::sprop() {
                        return Sort::sprop();
                    }

                    if Sort::prop() <= self {
                        return Sort::prop();
                    }
                }
                SortKind::Param(_) => (),
            }
        }

        Self {
            kind: SortKind::Trunc(Arc::new(self)),
        }
    }

    pub fn max(self, other: Self) -> Self {
        if ENABLE_NORMALIZATION && self <= other {
            return other;
        }

        if ENABLE_NORMALIZATION && other <= self {
            return self;
        }

        Self {
            kind: SortKind::Max(Arc::new(self), Arc::new(other)),
        }
    }

    pub(super) fn universe_params(&self) -> UniverseParams {
        match &self.kind {
            SortKind::SProp | SortKind::Prop | SortKind::Type => Default::default(),
            SortKind::Param(i) => UniverseParams {
                sort_vars: *i + 1,
                level_vars: 0,
            },
            SortKind::Trunc(sort) => sort.universe_params(),
            SortKind::Max(a, b) => a.universe_params().max(b.universe_params()),
        }
    }

    fn fast_eq(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (SortKind::Param(l0), SortKind::Param(r0)) => l0 == r0,
            (SortKind::Trunc(l0), SortKind::Trunc(r0)) => l0.fast_eq(r0),
            (SortKind::Max(l0, l1), SortKind::Max(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            _ => core::mem::discriminant(&self.kind) == core::mem::discriminant(&other.kind),
        }
    }

    pub fn shallow_eq(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (SortKind::Param(l0), SortKind::Param(r0)) => l0 == r0,
            (SortKind::Trunc(l0), SortKind::Trunc(r0)) => Arc::ptr_eq(l0, r0),
            (SortKind::Max(l0, l1), SortKind::Max(r0, r1)) => {
                Arc::ptr_eq(l0, r0) && Arc::ptr_eq(l1, r1)
            }
            _ => core::mem::discriminant(&self.kind) == core::mem::discriminant(&other.kind),
        }
    }

    fn prop_le(&self) -> bool {
        match &self.kind {
            SortKind::SProp | SortKind::Param(_) => false,
            SortKind::Prop | SortKind::Type => true,
            SortKind::Trunc(sort) => sort.prop_le(),
            SortKind::Max(a, b) => a.prop_le() || b.prop_le(),
        }
    }

    fn type_le(&self) -> bool {
        match &self.kind {
            SortKind::SProp | SortKind::Prop | SortKind::Param(_) | SortKind::Trunc(_) => false,
            SortKind::Type => true,
            SortKind::Max(a, b) => a.type_le() || b.type_le(),
        }
    }

    pub fn trunc_le(&self, other: &Self) -> bool {
        match &self.kind {
            SortKind::SProp => true,
            SortKind::Prop | SortKind::Type => other.prop_le(),
            SortKind::Max(a, b) => a.trunc_le(other) && b.trunc_le(other),
            SortKind::Trunc(a) => a.trunc_le(other),
            SortKind::Param(i) => match &other.kind {
                SortKind::SProp => false,
                SortKind::Trunc(b) => self.trunc_le(b),
                SortKind::Prop | SortKind::Type => true,
                SortKind::Param(j) => i == j,
                SortKind::Max(a, b) => self.trunc_le(a) || self.trunc_le(b),
            },
        }
    }

    pub fn le(&self, other: &Self) -> bool {
        match &self.kind {
            SortKind::SProp => true,
            SortKind::Prop => other.prop_le(),
            SortKind::Type => other.type_le(),
            SortKind::Max(a, b) => (**a).le(other) && (**b).le(other),
            SortKind::Trunc(a) => a.trunc_le(other),
            SortKind::Param(i) => match &other.kind {
                SortKind::SProp | SortKind::Prop | SortKind::Trunc(_) => false,
                SortKind::Type => true,
                SortKind::Param(j) => i == j,
                SortKind::Max(a, b) => self.le(a) || self.le(b),
            },
        }
    }

    fn le_prop(&self) -> bool {
        match &self.kind {
            SortKind::SProp | SortKind::Prop | SortKind::Trunc(_) => true,
            SortKind::Type | SortKind::Param(_) => false,
            SortKind::Max(a, b) => (**a).le_prop() && (**b).le_prop(),
        }
    }
}

impl core::ops::Add<u16> for Level {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u16) -> Self::Output {
        self.add(rhs)
    }
}

impl Level {
    pub fn zero() -> Self {
        Self {
            minv: 0,
            maxv: 0,
            kind: LevelKind::Zero,
        }
    }

    pub fn param(i: u16) -> Self {
        Self {
            minv: 0,
            maxv: 0,
            kind: LevelKind::Param(i),
        }
    }

    pub fn succ(self) -> Self {
        self.add(1)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, n: u16) -> Self {
        if ENABLE_NORMALIZATION && n == 0 {
            return self;
        }

        if ENABLE_NORMALIZATION
            && let LevelKind::Add(_, ref mut s) = self.kind
            && *s <= u16::MAX - n
        {
            let maxv = self
                .maxv
                .checked_add(i32::from(n))
                .unwrap_or_else(|| panic!("Tried to nest too many levels!"));

            *s += n;
            self.minv += i32::from(n);
            self.maxv = maxv;
            return self;
        }

        let maxv = self
            .maxv
            .checked_add(i32::from(n))
            .unwrap_or_else(|| panic!("Tried to nest too many levels!"));

        Self {
            minv: self.minv + i32::from(n),
            maxv,
            kind: LevelKind::Add(self.into(), n),
        }
    }

    pub fn max(self, other: Self) -> Self {
        if ENABLE_NORMALIZATION && self <= other {
            return other;
        }

        if ENABLE_NORMALIZATION && other <= self {
            return self;
        }

        let minv = self.minv.max(other.minv);
        let maxv = self.maxv.max(other.maxv);

        let kind = LevelKind::Max(Arc::new(self), Arc::new(other));

        Self { minv, maxv, kind }
    }

    pub fn when(self, cond: Sort) -> Self {
        let minv = if ENABLE_NORMALIZATION && cond.le_prop() {
            return Self::zero();
        } else if cond.type_le() {
            if ENABLE_NORMALIZATION {
                return self;
            } else {
                self.minv
            }
        } else {
            0
        };

        Self {
            minv,
            maxv: self.maxv,
            kind: LevelKind::When(Arc::new(cond), Arc::new(self)),
        }
    }

    pub(super) fn universe_params(&self) -> UniverseParams {
        match &self.kind {
            LevelKind::Zero => Default::default(),
            LevelKind::Param(i) => UniverseParams {
                sort_vars: 0,
                level_vars: *i + 1,
            },
            LevelKind::Add(a, _) => a.universe_params(),
            LevelKind::When(a, b) => a.universe_params().max(b.universe_params()),
            LevelKind::Max(a, b) => a.universe_params().max(b.universe_params()),
        }
    }

    fn fast_eq(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (LevelKind::Param(l0), LevelKind::Param(r0)) => l0 == r0,
            (LevelKind::Add(l0, n), LevelKind::Add(r0, m)) => n == m && l0.fast_eq(r0),
            (LevelKind::When(l0, l1), LevelKind::When(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            (LevelKind::Max(l0, l1), LevelKind::Max(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            (a, b) => core::mem::discriminant(a) == core::mem::discriminant(b),
        }
    }

    pub fn shallow_eq(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (LevelKind::Param(l0), LevelKind::Param(r0)) => l0 == r0,
            (LevelKind::Add(l0, n), LevelKind::Add(r0, m)) => n == m && Arc::ptr_eq(l0, r0),
            (LevelKind::When(l0, l1), LevelKind::When(r0, r1)) => {
                Arc::ptr_eq(l0, r0) && Arc::ptr_eq(l1, r1)
            }
            (LevelKind::Max(l0, l1), LevelKind::Max(r0, r1)) => {
                Arc::ptr_eq(l0, r0) && Arc::ptr_eq(l1, r1)
            }
            (a, b) => core::mem::discriminant(a) == core::mem::discriminant(b),
        }
    }

    fn le_(&self, other: &Self, balance: i32) -> bool {
        match (&self.kind, &other.kind) {
            (LevelKind::Zero, _) => balance + other.minv >= 0,
            (LevelKind::Param(_), LevelKind::Zero) => false,
            (LevelKind::Param(a), LevelKind::Param(b)) => a == b && balance >= 0,
            (LevelKind::Param(_), LevelKind::When(k, a)) => k.type_le() && self.le_(a, balance),

            (LevelKind::Add(a, n), LevelKind::Add(b, m)) => {
                a.le_(b, balance + i32::from(*m) - i32::from(*n))
            }
            (LevelKind::Add(a, n), _) => a.le_(other, balance - i32::from(*n)),
            (_, LevelKind::Add(b, n)) => self.le_(b, balance + i32::from(*n)),

            (LevelKind::Max(a, b), _) => a.le_(other, balance) && b.le_(other, balance),
            (_, LevelKind::Max(a, b)) => self.le_(a, balance) || self.le_(b, balance),

            (LevelKind::When(sa, a), LevelKind::When(sb, b)) => {
                if sa.le_prop() {
                    return balance + other.minv >= 0;
                }

                let monotone = **sa <= **sb && (balance + other.minv >= 0 || sa.type_le());
                a.le_(if monotone { b } else { other }, balance)
            }
            (LevelKind::When(s, a), _) => {
                if s.le_prop() {
                    balance + other.minv >= 0
                } else {
                    a.le_(other, balance)
                }
            }
        }
    }

    pub fn le(&self, other: &Self) -> bool {
        self.le_(other, 0)
    }
}

impl Universe {
    pub fn le(&self, other: &Self) -> bool {
        self.sort <= other.sort && (self.sort.le_prop() || self.level.le(&other.level))
    }

    pub fn succ(&self) -> Self {
        Self {
            sort: self.sort.clone(),
            level: self.level.clone().succ(),
        }
    }

    pub fn type_of(&self) -> Self {
        Self {
            sort: Sort::ty(),
            level: self.level.clone().succ().when(self.sort.clone()),
        }
    }

    pub fn shallow_eq(&self, other: &Self) -> bool {
        self.sort.shallow_eq(&other.sort) && self.level.shallow_eq(&other.level)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Level, Sort, Universe};

    #[test]
    fn test_sort_le() {
        assert!(!Sort::ty().le(&Sort::prop()));
        assert!(Sort::sprop().trunc().le(&Sort::param(0)));
        assert!(Sort::sprop().le(&Sort::param(0)));
        assert!(!Sort::param(0).le(&Sort::sprop()));
    }

    fn test_le_refl(l: Level) {
        assert!(l.le(&l), "REFL {l:?}");
    }

    #[test]
    fn test_level_le() {
        test_le_refl(Level::zero().succ().when(Sort::ty()));
        test_le_refl(Level::zero().max(Level::zero().succ()));
        assert!(Level::zero().le(&Level::param(0).succ().when(Sort::ty())));
        assert!(Level::zero().le(&Level::param(0).succ().when(Sort::prop())));
        assert!(
            Level::zero()
                .when(Sort::ty())
                .le(&Level::param(0).succ().when(Sort::prop()))
        );
    }

    #[test]
    fn test_univ_eq() {
        assert_eq!(
            Universe {
                sort: Sort::param(8).trunc(),
                level: Level::zero()
            },
            Universe {
                sort: Sort::param(8).trunc(),
                level: Level::param(9).when(Sort::ty())
            }
        )
    }
}
