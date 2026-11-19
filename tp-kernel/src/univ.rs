use std::cmp::Ordering;

use triomphe::Arc;

#[derive(Debug, Clone)]
pub enum Sort {
    SProp,
    Prop,
    Type,
    Param(u32),
    Trunc(Arc<Sort>),
    Max(Arc<Sort>, Arc<Sort>),
}

#[derive(Debug, Clone)]
pub struct Level {
    kind: LevelKind,
    minv: i32,
}

#[derive(Debug, Clone)]
enum LevelKind {
    Zero,
    Param(u32),
    Succ(Arc<Level>),
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
        match (self, other) {
            (Sort::SProp, Sort::SProp) | (Sort::Prop, Sort::Prop) | (Sort::Type, Sort::Type) => {
                return Some(std::cmp::Ordering::Equal);
            }
            (Sort::Param(i), Sort::Param(j)) => {
                return if i == j { Some(Ordering::Equal) } else { None };
            }
            (Sort::Param(_), Sort::Prop) | (Sort::Prop, Sort::Param(_)) => return None,
            (Sort::SProp, Sort::Param(_))
            | (Sort::Param(_), Sort::Type)
            | (Sort::SProp, Sort::Prop)
            | (Sort::SProp, Sort::Type)
            | (Sort::Prop, Sort::Type) => {
                return Some(Ordering::Less);
            }
            (Sort::Param(_), Sort::SProp)
            | (Sort::Type, Sort::Param(_))
            | (Sort::Prop, Sort::SProp)
            | (Sort::Type, Sort::SProp)
            | (Sort::Type, Sort::Prop) => {
                return Some(Ordering::Greater);
            }
            (Sort::Max(_, _), _)
            | (_, Sort::Max(_, _))
            | (Sort::Trunc(_), _)
            | (_, Sort::Trunc(_)) => (),
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
        match (self.sort <= Sort::Prop, other.sort <= Sort::Prop) {
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
    pub fn trunc(self) -> Self {
        Self::Trunc(Arc::new(self))
    }

    pub fn max(self, other: Self) -> Self {
        Self::Max(Arc::new(self), Arc::new(other))
    }

    fn fast_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Param(l0), Self::Param(r0)) => l0 == r0,
            (Self::Trunc(l0), Self::Trunc(r0)) => l0.fast_eq(r0),
            (Self::Max(l0, l1), Self::Max(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }

    fn prop_le(&self) -> bool {
        match self {
            Sort::SProp | Sort::Param(_) => false,
            Sort::Prop | Sort::Type => true,
            Sort::Trunc(sort) => sort.prop_le(),
            Sort::Max(a, b) => a.prop_le() || b.prop_le(),
        }
    }

    fn type_le(&self) -> bool {
        match self {
            Sort::SProp | Sort::Prop | Sort::Param(_) | Sort::Trunc(_) => false,
            Sort::Type => true,
            Sort::Max(a, b) => a.type_le() || b.type_le(),
        }
    }

    pub fn trunc_le(&self, other: &Self) -> bool {
        match self {
            Sort::SProp => true,
            Sort::Prop | Sort::Type => other.prop_le(),
            Sort::Max(a, b) => a.trunc_le(other) && b.trunc_le(other),
            Sort::Trunc(a) => a.trunc_le(other),
            Sort::Param(i) => match other {
                Sort::SProp => false,
                Sort::Trunc(b) => self.trunc_le(b),
                Sort::Prop | Sort::Type => true,
                Sort::Param(j) => i == j,
                Sort::Max(a, b) => self.trunc_le(a) || self.trunc_le(b),
            },
        }
    }

    pub fn le(&self, other: &Self) -> bool {
        match self {
            Sort::SProp => true,
            Sort::Prop => other.prop_le(),
            Sort::Type => other.type_le(),
            Sort::Max(a, b) => (**a).le(other) && (**b).le(other),
            Sort::Trunc(a) => a.trunc_le(other),
            Sort::Param(i) => match other {
                Sort::SProp | Sort::Prop | Sort::Trunc(_) => false,
                Sort::Type => true,
                Sort::Param(j) => i == j,
                Sort::Max(a, b) => self.le(a) || self.le(b),
            },
        }
    }

    fn le_prop(&self) -> bool {
        match self {
            Sort::SProp | Sort::Prop | Sort::Trunc(_) => true,
            Sort::Type | Sort::Param(_) => false,
            Sort::Max(a, b) => (**a).le_prop() && (**b).le_prop(),
        }
    }
}

impl Level {
    pub fn zero() -> Self {
        Self {
            minv: 0,
            kind: LevelKind::Zero,
        }
    }

    pub fn param(i: u32) -> Self {
        Self {
            minv: 0,
            kind: LevelKind::Param(i),
        }
    }

    pub fn succ(self) -> Self {
        Self {
            minv: self.minv + 1,
            kind: LevelKind::Succ(self.into()),
        }
    }

    pub fn max(self, other: Self) -> Self {
        let minv = self.minv.max(other.minv);
        let kind = match (self.kind, other.kind) {
            (LevelKind::Zero, a) | (a, LevelKind::Zero) if !cfg!(tp_disable_noramlize) => a,
            (s, o) => LevelKind::Max(
                Arc::new(Self {
                    minv: self.minv,
                    kind: s,
                }),
                Arc::new(Self {
                    minv: other.minv,
                    kind: o,
                }),
            ),
        };

        Self { minv, kind }
    }

    pub fn when(self, cond: Sort) -> Self {
        let minv = if !cfg!(tp_disable_noramlize) && cond <= Sort::Prop {
            return Self::zero();
        } else if Sort::Type <= cond {
            self.minv
        } else {
            0
        };

        Self {
            minv,
            kind: LevelKind::When(Arc::new(cond), Arc::new(self)),
        }
    }

    fn fast_eq(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (LevelKind::Param(l0), LevelKind::Param(r0)) => l0 == r0,
            (LevelKind::Succ(l0), LevelKind::Succ(r0)) => l0.fast_eq(r0),
            (LevelKind::When(l0, l1), LevelKind::When(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            (LevelKind::Max(l0, l1), LevelKind::Max(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            (a, b) => core::mem::discriminant(a) == core::mem::discriminant(b),
        }
    }

    fn le_(&self, other: &Self, balance: i32) -> bool {
        match (&self.kind, &other.kind) {
            (LevelKind::Zero, _) => balance + other.minv >= 0,
            (LevelKind::Succ(a), LevelKind::Succ(b)) => a.le_(b, balance),
            (LevelKind::Succ(a), _) => a.le_(other, balance - 1),
            (_, LevelKind::Succ(b)) => self.le_(b, balance + 1),
            (LevelKind::Max(a, b), _) => a.le_(other, balance) && b.le_(other, balance),
            (_, LevelKind::Max(a, b)) => self.le_(a, balance) || self.le_(b, balance),
            (LevelKind::When(sa, a), LevelKind::When(sb, b)) => {
                if **sa <= Sort::Prop && balance + other.minv >= 0 {
                    return true;
                }

                let monotone = **sa <= **sb && (balance + other.minv >= 0 || Sort::Type <= **sa);
                a.le_(if monotone { b } else { other }, balance)
            }
            (LevelKind::When(s, a), _) => {
                (**s <= Sort::Prop && balance + other.minv >= 0) || a.le_(other, balance)
            }
            (LevelKind::Param(_), LevelKind::Zero) => false,
            (LevelKind::Param(a), LevelKind::Param(b)) => a == b && balance >= 0,
            (LevelKind::Param(_), LevelKind::When(k, a)) => {
                Sort::Type <= **k && self.le_(a, balance)
            }
        }
    }

    pub fn le(&self, other: &Self) -> bool {
        self.le_(other, 0)
    }

    pub fn offset(mut self, offset: u16) -> Level {
        for _ in 0..offset {
            self = self.succ()
        }
        self
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
            sort: Sort::Type,
            level: self.level.clone().succ().when(self.sort.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Level, Sort, Universe};

    #[test]
    fn test_sort_le() {
        assert!(!Sort::Type.le(&Sort::Prop));
        assert!(Sort::SProp.trunc().le(&Sort::Param(0)));
        assert!(Sort::SProp.le(&Sort::Param(0)));
        assert!(!Sort::Param(0).le(&Sort::SProp));
    }

    fn test_le_refl(l: Level) {
        assert!(l.le(&l), "REFL {l:?}");
    }

    #[test]
    fn test_level_le() {
        test_le_refl(Level::zero().succ().when(Sort::Type));
        test_le_refl(Level::zero().max(Level::zero().succ()));
        assert!(Level::zero().le(&Level::param(0).succ().when(Sort::Type)));
        assert!(Level::zero().le(&Level::param(0).succ().when(Sort::Prop)));
        assert!(
            Level::zero()
                .when(Sort::Type)
                .le(&Level::param(0).succ().when(Sort::Prop))
        );
    }

    #[test]
    fn test_univ_eq() {
        assert_eq!(
            Universe {
                sort: Sort::Param(8).trunc(),
                level: Level::zero()
            },
            Universe {
                sort: Sort::Param(8).trunc(),
                level: Level::param(9).when(Sort::Type)
            }
        )
    }
}
