use std::cmp::Ordering;

use triomphe::Arc;

#[derive(Debug, Clone)]
pub enum Kind {
    SProp,
    Prop,
    Type,
    Param(u32),
    Trunc(Arc<Kind>),
    Max(Arc<Kind>, Arc<Kind>),
}

#[derive(Debug, Clone)]
pub enum Level {
    Zero,
    Param(u32),
    Succ(Arc<Level>),
    When(Arc<Kind>, Arc<Level>),
    Max(Arc<Level>, Arc<Level>),
}

#[derive(Debug, Clone)]
pub struct Universe {
    pub kind: Kind,
    pub level: Level,
}

impl Eq for Kind {}
impl PartialEq for Kind {
    fn eq(&self, other: &Self) -> bool {
        if self.fast_eq(other) {
            return true;
        }

        self <= other && other <= self
    }
}

impl PartialOrd for Kind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // handle all combinations of sprop, prop, type, and param
        // since they are easy and cheap to check, then fall back to
        // general alg
        match (self, other) {
            (Kind::SProp, Kind::SProp) | (Kind::Prop, Kind::Prop) | (Kind::Type, Kind::Type) => {
                return Some(std::cmp::Ordering::Equal);
            }
            (Kind::Param(i), Kind::Param(j)) => {
                return if i == j { Some(Ordering::Equal) } else { None };
            }
            (Kind::Param(_), Kind::Prop) | (Kind::Prop, Kind::Param(_)) => return None,
            (Kind::SProp, Kind::Param(_))
            | (Kind::Param(_), Kind::Type)
            | (Kind::SProp, Kind::Prop)
            | (Kind::SProp, Kind::Type)
            | (Kind::Prop, Kind::Type) => {
                return Some(Ordering::Less);
            }
            (Kind::Param(_), Kind::SProp)
            | (Kind::Type, Kind::Param(_))
            | (Kind::Prop, Kind::SProp)
            | (Kind::Type, Kind::SProp)
            | (Kind::Type, Kind::Prop) => {
                return Some(Ordering::Greater);
            }
            (Kind::Max(_, _), _)
            | (_, Kind::Max(_, _))
            | (Kind::Trunc(_), _)
            | (_, Kind::Trunc(_)) => (),
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
        match (self.kind <= Kind::Prop, other.kind <= Kind::Prop) {
            (true, true) => return self.kind == other.kind,
            (true, false) | (false, true) => return false,
            (false, false) => (),
        }

        if self.kind.fast_eq(&other.kind) && self.level.fast_eq(&other.level) {
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

impl Kind {
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
            Kind::SProp => true,
            Kind::Prop | Kind::Type | Kind::Param(_) => false,
            Kind::Trunc(kind) => kind.eq_sprop(),
            Kind::Max(a, b) => a.eq_sprop() && b.eq_sprop(),
        }
    }

    fn prop_le(&self) -> bool {
        match self {
            Kind::SProp | Kind::Param(_) => false,
            Kind::Prop | Kind::Type => true,
            Kind::Trunc(kind) => kind.prop_le(),
            Kind::Max(a, b) => a.prop_le() || b.prop_le(),
        }
    }

    fn type_le(&self) -> bool {
        match self {
            Kind::SProp | Kind::Prop | Kind::Param(_) | Kind::Trunc(_) => false,
            Kind::Type => true,
            Kind::Max(a, b) => a.type_le() || b.type_le(),
        }
    }

    pub fn trunc_le(&self, other: &Self) -> bool {
        match self {
            Kind::SProp => true,
            Kind::Prop | Kind::Type => other.prop_le(),
            Kind::Max(a, b) => a.trunc_le(other) && b.trunc_le(other),
            Kind::Trunc(a) => match other {
                Kind::SProp => a.eq_sprop(),
                Kind::Prop | Kind::Type => true,
                Kind::Param(_) => a.eq_sprop(),
                Kind::Trunc(b) => a.trunc_le(b),
                Kind::Max(a, b) => self.trunc_le(a) || self.trunc_le(b),
            },
            Kind::Param(i) => match other {
                Kind::SProp => false,
                Kind::Trunc(b) => self.trunc_le(b),
                Kind::Prop | Kind::Type => true,
                Kind::Param(j) => i == j,
                Kind::Max(a, b) => self.trunc_le(a) || self.trunc_le(b),
            },
        }
    }

    pub fn le(&self, other: &Self) -> bool {
        match self {
            Kind::SProp => true,
            Kind::Prop => other.prop_le(),
            Kind::Type => other.type_le(),
            Kind::Max(a, b) => (**a).le(other) && (**b).le(other),
            Kind::Trunc(a) => match other {
                Kind::SProp => a.eq_sprop(),
                Kind::Prop | Kind::Type => true,
                Kind::Param(_) => a.eq_sprop(),
                Kind::Trunc(b) => a.trunc_le(b),
                Kind::Max(a, b) => self.le(a) || self.le(b),
            },
            Kind::Param(i) => match other {
                Kind::SProp | Kind::Prop | Kind::Trunc(_) => false,
                Kind::Type => true,
                Kind::Param(j) => i == j,
                Kind::Max(a, b) => self.le(a) || self.le(b),
            },
        }
    }

    fn le_prop(&self) -> bool {
        match self {
            Kind::SProp | Kind::Prop | Kind::Trunc(_) => true,
            Kind::Type | Kind::Param(_) => false,
            Kind::Max(a, b) => (**a).le_prop() && (**b).le_prop(),
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

    pub fn when(self, cond: Kind) -> Self {
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
                balance >= 0 || Kind::Type <= **k && Level::Zero.le_(a, balance)
            }
            (Level::Param(_), Level::When(k, a)) => Kind::Type.le(k) && self.le_(a, balance),
            (Level::When(ka, a), Level::When(kb, b)) => {
                if **ka <= Kind::Prop {
                    return Level::Zero.le_(other, balance);
                }

                if **kb <= Kind::Prop {
                    return a.le_(&Level::Zero, balance);
                }

                a.le_(b, balance)
            }
            (Level::When(k, a), b) => {
                **k <= Kind::Prop && balance >= 0 || Kind::Type <= **k && a.le_(b, balance)
            }
        }
    }

    pub fn le(&self, other: &Self) -> bool {
        self.le_(other, 0)
    }
}

impl Universe {
    pub fn le(&self, other: &Self) -> bool {
        self.kind <= other.kind && (self.kind.le_prop() || self.level.le(&other.level))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Kind, Level, Universe};

    #[test]
    fn test_kind_le() {
        assert!(!Kind::Type.le(&Kind::Prop));
        assert!(Kind::SProp.trunc().le(&Kind::Param(0)));
        assert!(Kind::SProp.le(&Kind::Param(0)));
        assert!(!Kind::Param(0).le(&Kind::SProp));
    }

    fn test_le_refl(l: Level) {
        assert!(l.le(&l), "REFL {l:?}");
    }

    #[test]
    fn test_level_le() {
        test_le_refl(Level::Zero.succ().when(Kind::Type));
        test_le_refl(Level::Zero.max(Level::Zero.succ()));
        assert!(Level::Zero.le(&Level::Param(0).succ().when(Kind::Type)));
        assert!(Level::Zero.le(&Level::Param(0).succ().when(Kind::Prop)));
        assert!(
            Level::Zero
                .when(Kind::Type)
                .le(&Level::Param(0).succ().when(Kind::Prop))
        );
    }

    #[test]
    fn test_univ_eq() {
        assert_eq!(
            Universe {
                kind: Kind::Param(8).trunc(),
                level: Level::Zero
            },
            Universe {
                kind: Kind::Param(8).trunc(),
                level: Level::Param(9).when(Kind::Type)
            }
        )
    }
}
