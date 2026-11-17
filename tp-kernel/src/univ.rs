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
pub enum Level {
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

    fn eq_sprop(&self) -> bool {
        match self {
            Sort::SProp => true,
            Sort::Prop | Sort::Type | Sort::Param(_) => false,
            Sort::Trunc(sort) => sort.eq_sprop(),
            Sort::Max(a, b) => a.eq_sprop() && b.eq_sprop(),
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
            Sort::Trunc(a) => match other {
                Sort::SProp => a.eq_sprop(),
                Sort::Prop | Sort::Type => true,
                Sort::Param(_) => a.eq_sprop(),
                Sort::Trunc(b) => a.trunc_le(b),
                Sort::Max(a, b) => self.trunc_le(a) || self.trunc_le(b),
            },
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
            Sort::Trunc(a) => match other {
                Sort::SProp => a.eq_sprop(),
                Sort::Prop | Sort::Type => true,
                Sort::Param(_) => a.eq_sprop(),
                Sort::Trunc(b) => a.trunc_le(b),
                Sort::Max(a, b) => self.le(a) || self.le(b),
            },
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
    pub fn succ(self) -> Self {
        Self::Succ(Arc::new(self))
    }

    pub fn max(self, other: Self) -> Self {
        Self::Max(Arc::new(self), Arc::new(other))
    }

    pub fn when(self, cond: Sort) -> Self {
        Self::When(Arc::new(cond), Arc::new(self))
    }

    fn fast_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Param(l0), Self::Param(r0)) => l0 == r0,
            (Self::Succ(l0), Self::Succ(r0)) => l0.fast_eq(r0),
            (Self::When(l0, l1), Self::When(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            (Self::Max(l0, l1), Self::Max(r0, r1)) => l0.fast_eq(r0) && l1.fast_eq(r1),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }

    fn le_(&self, other: &Self, balance: i64) -> bool {
        match (self, other) {
            (Level::Zero, Level::Zero | Level::Param(_)) => balance >= 0,
            (Level::Zero, _) if balance >= 0 => true,
            (Level::Succ(a), Level::Succ(b)) => a.le_(b, balance),
            (Level::Succ(a), b) => a.le_(b, balance - 1),
            (a, Level::Succ(b)) => a.le_(b, balance + 1),

            (Level::Max(a, b), k) => a.le_(k, balance) && b.le_(k, balance),
            (k, Level::Max(a, b)) => k.le_(a, balance) || k.le_(b, balance),

            (Level::Param(_), Level::Zero) => false,
            (Level::Param(i), Level::Param(j)) => i == j && balance >= 0,
            (Level::Zero, Level::When(k, a)) => {
                balance >= 0 || Sort::Type <= **k && Level::Zero.le_(a, balance)
            }
            (Level::Param(_), Level::When(k, a)) => Sort::Type.le(k) && self.le_(a, balance),
            (Level::When(ka, a), Level::When(kb, b)) => {
                if **ka <= Sort::Prop {
                    return Level::Zero.le_(other, balance);
                }

                if **kb <= Sort::Prop {
                    return a.le_(&Level::Zero, balance);
                }

                a.le_(b, balance)
            }
            (Level::When(k, a), b) => {
                **k <= Sort::Prop && balance >= 0 || Sort::Type <= **k && a.le_(b, balance)
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
        test_le_refl(Level::Zero.succ().when(Sort::Type));
        test_le_refl(Level::Zero.max(Level::Zero.succ()));
        assert!(Level::Zero.le(&Level::Param(0).succ().when(Sort::Type)));
        assert!(Level::Zero.le(&Level::Param(0).succ().when(Sort::Prop)));
        assert!(
            Level::Zero
                .when(Sort::Type)
                .le(&Level::Param(0).succ().when(Sort::Prop))
        );
    }

    #[test]
    fn test_univ_eq() {
        assert_eq!(
            Universe {
                sort: Sort::Param(8).trunc(),
                level: Level::Zero
            },
            Universe {
                sort: Sort::Param(8).trunc(),
                level: Level::Param(9).when(Sort::Type)
            }
        )
    }
}
