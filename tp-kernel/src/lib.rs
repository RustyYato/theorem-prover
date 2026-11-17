pub mod univ;

use std::mem::DropGuard;

use triomphe::Arc;
pub use univ::{Level, Sort, Universe};
use ustr::Ustr;

#[derive(Clone, PartialEq, Eq)]
pub struct Term {
    repr: TermRepr,
}

impl core::fmt::Debug for Term {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.repr.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TermRepr {
    Var(usize),
    Global(ustr::Ustr),
    Univ(Universe),
    Lambda(Arc<[Term; 2]>),
    Forall(Arc<[Term; 2]>),
    Application(Arc<[Term; 2]>),
}

pub struct Context {
    types: Vec<Term>,
    globals: ustr::UstrMap<Global>,
}

struct Global {
    def: Term,
    ty: Term,
}

impl Term {
    pub fn var(id: usize) -> Self {
        Self {
            repr: TermRepr::Var(id),
        }
    }

    pub fn global(id: Ustr) -> Self {
        Self {
            repr: TermRepr::Global(id),
        }
    }

    pub fn univ(u: Universe) -> Self {
        Self {
            repr: TermRepr::Univ(u),
        }
    }

    pub fn lambda(arg_ty: Term, body: Term) -> Self {
        Self {
            repr: TermRepr::Lambda(Arc::new([arg_ty, body])),
        }
    }

    pub fn forall(arg_ty: Term, body: Term) -> Self {
        Self {
            repr: TermRepr::Forall(Arc::new([arg_ty, body])),
        }
    }

    pub fn apply(self, arg: Term) -> Self {
        Self {
            repr: TermRepr::Application(Arc::new([self, arg])),
        }
    }

    pub fn is_univ(&self) -> bool {
        matches!(self.repr, TermRepr::Univ(_))
    }

    fn weaken_by(&self, var: usize, skipped_binders: usize) {
        //
    }

    fn subst_(&self, var: usize, subst: &Self, skipped_binders: usize) -> Self {
        todo!()
    }

    fn subst(&self, var: usize, subst: &Self) -> Self {
        self.subst_(var, subst, 0)
    }

    fn is_defeq(&self, other: &Self, ctx: &mut Context) -> bool {
        let len = ctx.types.len();
        let _guard = DropGuard::new(ctx, |ctx| ctx.types.truncate(len));

        self == other
    }

    pub fn type_of(&self, ctx: &mut Context) -> Option<Term> {
        let len = ctx.types.len();
        let mut guard = DropGuard::new(ctx, |ctx| ctx.types.truncate(len));

        self.type_(&mut guard)
    }

    pub fn type_(&self, ctx: &mut Context) -> Option<Term> {
        match self.repr {
            TermRepr::Var(idx) => ctx.types.get(idx).cloned(),
            TermRepr::Global(name) => ctx.globals.get(&name).map(|x| x.ty.clone()),
            TermRepr::Univ(ref universe) => Some(Term {
                repr: TermRepr::Univ(universe.type_of()),
            }),
            TermRepr::Lambda(ref lam) => {
                let [arg_ty, body] = &**lam;

                let arg_sort = arg_ty.type_(ctx)?;

                if !arg_sort.is_univ() {
                    // arg_ty must be a type!
                    return None;
                }

                ctx.types.push(arg_ty.clone());
                let body_ty = body.type_(ctx)?;
                ctx.types.pop();

                Some(Term {
                    repr: TermRepr::Forall(Arc::new([arg_ty.clone(), body_ty])),
                })
            }
            TermRepr::Forall(ref forall) => {
                let [arg_ty, body_ty] = &**forall;

                let arg_sort = arg_ty.type_(ctx)?;
                ctx.types.push(arg_ty.clone());
                let body_sort = body_ty.type_(ctx)?;
                ctx.types.pop();

                let u = match (arg_sort.repr, body_sort.repr) {
                    (TermRepr::Univ(a), TermRepr::Univ(b)) => Universe {
                        sort: b.sort,
                        level: a.level.when(a.sort).max(b.level),
                    },
                    _ => return None,
                };

                Some(Term {
                    repr: TermRepr::Univ(u),
                })
            }
            TermRepr::Application(ref app) => {
                let [func, arg] = &**app;

                let func_ty = func.type_(ctx)?;
                let app_arg_ty = arg.type_(ctx)?;

                let (arg_ty, ret_ty) = match func_ty.repr {
                    TermRepr::Forall(ref forall) => {
                        let [arg_ty, ret_ty] = &**forall;

                        (arg_ty, ret_ty)
                    }
                    _ => return None,
                };

                if !arg_ty.is_defeq(&app_arg_ty, ctx) {
                    return None;
                }

                Some(ret_ty.subst(0, &app_arg_ty))
            }
        }
    }
}
