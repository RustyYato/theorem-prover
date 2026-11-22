use std::num::NonZero;

pub use erasable::Thin;
use ptr_union::Union8;
pub use slice_dst::SliceWithHeader;
use ustr::{Ustr, UstrSet};

#[derive(Clone, Copy)]
pub struct Ident {
    pub id: NonZero<u32>,
    pub val: Ustr,
}

impl core::fmt::Debug for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ident")
            .field("id", &self.id)
            .field("val", &self.val.as_str())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub globals: Vec<Global>,
}

#[derive(Debug, Clone)]
pub enum Global {
    Definition(Definition),
    Opaque(Definition),
    Axiom(Axiom),
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub name: Ident,
    pub sorts: Vec<Ustr>,
    pub levels: Vec<Ustr>,
    pub ty: Term,
    pub expr: Term,
}

#[derive(Debug, Clone)]
pub struct Axiom {
    pub name: Ident,
    pub sorts: Vec<Ustr>,
    pub levels: Vec<Ustr>,
    pub ty: Term,
}

#[derive(Clone)]
pub enum Term {
    Variable(Ident),
    GlobalRef(Box<GlobalRef>),
    Univ(Universe),
    Forall(Thin<Box<SliceWithHeader<Term, Argument>>>),
    Lambda(Thin<Box<SliceWithHeader<Term, Argument>>>),
    Application(Thin<Box<SliceWithHeader<Term, Term>>>),
    Natural(u64),
}

#[derive(Clone, Debug)]
pub struct GlobalRef {
    pub ident: Ident,
    pub sorts: Box<[Sort]>,
    pub levels: Box<[Level]>,
}

impl core::fmt::Debug for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(arg0) => f.debug_tuple("Variable").field(arg0).finish(),
            Self::Univ(arg0) => f.debug_tuple("Univ").field(arg0).finish(),
            Self::GlobalRef(arg0) => f.debug_tuple("GlobalRef").field(arg0).finish(),
            Self::Forall(arg0) => f
                .debug_struct("Forall")
                .field("args", &&arg0.slice)
                .field("body", &arg0.header)
                .finish(),
            Self::Lambda(arg0) => f
                .debug_struct("Lambda")
                .field("args", &&arg0.slice)
                .field("body", &arg0.header)
                .finish(),
            Self::Application(arg0) => f
                .debug_struct("Application")
                .field("func", &&arg0.slice)
                .field("args", &arg0.header)
                .finish(),
            Self::Natural(arg0) => f.debug_tuple("Natural").field(arg0).finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Forall {
    pub args: Vec<Argument>,
    pub body: Term,
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub name: Ident,
    pub ty: Term,
}

#[derive(Debug, Clone)]
pub struct Universe {
    pub sort: Sort,
    pub level: Level,
}

#[derive(Clone, Copy)]
pub enum UnpackedSort<'a> {
    SProp,
    Prop,
    Type,
    Param(Ident),
    Trunc(&'a Sort),
    Max(&'a [Sort]),
}

#[derive(Clone, Copy)]
pub enum UnpackedLevel<'a> {
    Nat(u16),
    Param(Ident, u16),
    Add(&'a Level, u16),
    When(&'a Sort, &'a Level),
    Max(&'a [Level]),
}

#[derive(Clone)]
pub struct Sort {
    #[allow(clippy::type_complexity)]
    data: Union8<
        &'static VarData, // SProp
        &'static VarData, // Prop
        &'static VarData, // Type
        Box<Ident>,
        Box<Self>,
        Box<SliceWithHeader<(), Self>>,
        &'static !,
        &'static !,
    >,
}

impl core::fmt::Debug for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.unpack().fmt(f)
    }
}

#[derive(Clone)]
pub struct Level {
    #[allow(clippy::type_complexity)]
    data: Union8<
        &'static VarData,
        Box<(Ustr, NonZero<u32>, u16)>,
        Box<(Level, u16)>,
        Box<(Sort, Self)>,
        Box<SliceWithHeader<(), Self>>,
        &'static !,
        &'static !,
        &'static !,
    >,
}

impl core::fmt::Debug for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.unpack().fmt(f)
    }
}

impl core::fmt::Debug for UnpackedSort<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SProp => write!(f, "SProp"),
            Self::Prop => write!(f, "Prop"),
            Self::Type => write!(f, "Type"),
            Self::Param(arg0) => f.debug_tuple("Param").field(arg0).finish(),
            Self::Trunc(arg0) => f.debug_tuple("Trunc").field(arg0).finish(),
            Self::Max(arg0) => {
                let mut f = f.debug_tuple("Max");
                for a in arg0.iter() {
                    f.field(a);
                }
                f.finish()
            }
        }
    }
}

impl core::fmt::Debug for UnpackedLevel<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nat(arg0) => f.debug_tuple("Nat").field(arg0).finish(),
            Self::Param(arg0, arg1) => f.debug_tuple("Param").field(arg0).field(arg1).finish(),
            Self::Add(arg0, arg1) => f.debug_tuple("Add").field(arg0).field(arg1).finish(),
            Self::When(arg0, arg1) => f.debug_tuple("When").field(arg0).field(arg1).finish(),
            Self::Max(arg0) => {
                let mut f = f.debug_tuple("Max");
                for a in arg0.iter() {
                    f.field(a);
                }
                f.finish()
            }
        }
    }
}

#[derive(Debug)]
#[repr(align(8))]
struct VarData(core::marker::PhantomPinned);

impl Sort {
    const CONST: &'static VarData = unsafe { &*core::ptr::with_exposed_provenance(1 << 8) };

    pub fn unpack(&self) -> UnpackedSort<'_> {
        match unsafe { self.data.as_deref_unchecked().unpack() } {
            ptr_union::Enum8::G(x) | ptr_union::Enum8::H(x) => *x,
            ptr_union::Enum8::A(_) => UnpackedSort::SProp,
            ptr_union::Enum8::B(_) => UnpackedSort::Prop,
            ptr_union::Enum8::C(_) => UnpackedSort::Type,
            ptr_union::Enum8::D(val) => UnpackedSort::Param(*val),
            ptr_union::Enum8::E(val) => UnpackedSort::Trunc(val),
            ptr_union::Enum8::F(val) => UnpackedSort::Max(&val.slice),
        }
    }

    pub fn sprop() -> Self {
        Self {
            data: unsafe { Union8::new_a(Self::CONST).unwrap_unchecked() },
        }
    }

    pub fn prop() -> Self {
        Self {
            data: unsafe { Union8::new_b(Self::CONST).unwrap_unchecked() },
        }
    }

    pub fn type_() -> Self {
        Self {
            data: unsafe { Union8::new_c(Self::CONST).unwrap_unchecked() },
        }
    }

    pub fn param(name: Ident) -> Self {
        Self {
            data: unsafe { Union8::new_d(Box::new(name)).unwrap_unchecked() },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn trunc(self) -> Self {
        Self {
            data: unsafe { Union8::new_e(Box::new(self)).unwrap_unchecked() },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn max<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Self>,
        I::IntoIter: ExactSizeIterator,
    {
        Self {
            data: unsafe { Union8::new_f(SliceWithHeader::new((), iter)).unwrap_unchecked() },
        }
    }
}

impl Level {
    pub fn unpack(&self) -> UnpackedLevel<'_> {
        match unsafe { self.data.as_deref_unchecked().unpack() } {
            ptr_union::Enum8::F(x) | ptr_union::Enum8::G(x) | ptr_union::Enum8::H(x) => *x,
            ptr_union::Enum8::A(a) => {
                let val = (core::ptr::from_ref(a).addr() >> 16) as u16;
                UnpackedLevel::Nat(val)
            }
            ptr_union::Enum8::B(&(val, id, offset)) => {
                UnpackedLevel::Param(Ident { id, val }, offset)
            }
            ptr_union::Enum8::C(val) => UnpackedLevel::Add(&val.0, val.1),
            ptr_union::Enum8::D(val) => UnpackedLevel::When(&val.0, &val.1),
            ptr_union::Enum8::E(val) => UnpackedLevel::Max(&val.slice),
        }
    }

    pub fn nat(val: u16) -> Self {
        let val = usize::from(val) << 16;
        let data: &VarData = unsafe { &*core::ptr::with_exposed_provenance(val | (1 << 8)) };
        Self {
            data: unsafe { Union8::new_a(data).unwrap_unchecked() },
        }
    }

    pub fn param(name: Ident, offset: u16) -> Self {
        Self {
            data: unsafe {
                Union8::new_b(Box::new((name.val, name.id, offset))).unwrap_unchecked()
            },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(self, offset: u16) -> Self {
        Self {
            data: unsafe { Union8::new_c(Box::new((self, offset))).unwrap_unchecked() },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn when(self, sort: Sort) -> Self {
        Self {
            data: unsafe { Union8::new_d(Box::new((sort, self))).unwrap_unchecked() },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn max<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Self>,
        I::IntoIter: ExactSizeIterator,
    {
        Self {
            data: unsafe { Union8::new_e(SliceWithHeader::new((), iter)).unwrap_unchecked() },
        }
    }
}

#[cfg(target_pointer_width = "16")]
compile_error!("16-bit targets are not supported");

// #[cfg(target_pointer_width = "32")]
// const _: () = assert!(core::mem::size_of::<Level>() == 8);

// #[cfg(target_pointer_width = "64")]
// const _: () = assert!(core::mem::size_of::<Level>() == 16);

#[test]
fn test() {
    for i in 0..=u16::MAX {
        Level::nat(i);
    }
}
