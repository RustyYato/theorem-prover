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

#[derive(Clone, PartialEq, Eq)]
enum TermRepr {
    Var(usize),
    Global(ustr::Ustr),
    Univ(Universe),
    Lambda(Arc<[Term; 2]>),
    Forall(Arc<[Term; 2]>),
    Application(Arc<[Term; 2]>),
}

impl core::fmt::Debug for TermRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(arg0) => f.debug_tuple("Var").field(arg0).finish(),
            Self::Global(arg0) => f.debug_tuple("Global").field(arg0).finish(),
            Self::Univ(arg0) => f.debug_tuple("Univ").field(arg0).finish(),
            Self::Lambda(arg0) => f
                .debug_struct("Lambda")
                .field("arg_ty", &arg0[0])
                .field("body", &arg0[1])
                .finish(),
            Self::Forall(arg0) => f
                .debug_struct("Forall")
                .field("arg_ty", &arg0[0])
                .field("body", &arg0[1])
                .finish(),
            Self::Application(arg0) => f.debug_tuple("Application").field(arg0).finish(),
        }
    }
}

#[derive(Default, Debug)]
pub struct Context {
    types: Vec<Term>,
    globals: ustr::UstrMap<Global>,
}

#[derive(Debug)]
enum Global {
    Definition { def: Term, ty: Term },
    Opaque { def: Term, ty: Term },
    Axiom { ty: Term },
}

#[derive(Debug)]
pub enum TypeError {
    UnboundLocalVariable,
    ExpectedUniv { found: Term },
    MissingGlobalVariable,
    ExpectedForall { found: Term },
    ExpectedDefeq { expected: Term, found: Term },
    //
}

impl Context {
    pub fn add_definition(&mut self, name: Ustr, term: Term, ty: Term) {
        let t = term.type_of(self);
        if !t.as_ref().is_ok_and(|t| t.is_defeq(&ty, self)) {
            eprintln!("{ty:#?}");
            eprintln!("{t:#?}");
            eprintln!("{term:#?}");
            panic!(
                "type of term is not equal, expected: {ty:?}, for term: {term:?} which has type: {t:?}"
            )
        }

        if !ty.type_of(self).is_ok_and(|t| t.is_univ()) {
            panic!("expected a type but found: {ty:?}")
        }

        match self.globals.entry(name) {
            std::collections::hash_map::Entry::Occupied(_) => {
                panic!("duplicate global name")
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Global::Definition { def: term, ty });
            }
        }
    }

    pub fn add_opaque(&mut self, name: Ustr, term: Term, ty: Term) {
        let t = term.type_of(self);
        if !t.as_ref().is_ok_and(|t| t.is_defeq(&ty, self)) {
            eprintln!("{ty:#?}");
            eprintln!("{t:#?}");
            eprintln!("{term:#?}");
            panic!(
                "type of term is not equal, expected: {ty:?}, for term: {term:?} which has type: {t:?}"
            )
        }

        if !ty.type_of(self).is_ok_and(|t| t.is_univ()) {
            panic!("expected a type but found: {ty:?}")
        }

        match self.globals.entry(name) {
            std::collections::hash_map::Entry::Occupied(_) => {
                panic!("duplicate global name")
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Global::Opaque { def: term, ty });
            }
        }
    }

    pub fn add_axiom(&mut self, name: Ustr, ty: Term) {
        if !ty.type_of(self).is_ok_and(|t| t.is_univ()) {
            panic!("expected a type but found: {ty:?}")
        }

        match self.globals.entry(name) {
            std::collections::hash_map::Entry::Occupied(_) => {
                panic!("duplicate global name")
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Global::Axiom { ty });
            }
        }
    }
}

impl Global {
    fn ty(&self) -> &Term {
        match self {
            Global::Definition { def: _, ty }
            | Global::Opaque { def: _, ty }
            | Global::Axiom { ty } => ty,
        }
    }
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

    fn shallow_eq(&self, other: &Self) -> bool {
        match (&self.repr, &other.repr) {
            (TermRepr::Var(i), TermRepr::Var(j)) => i == j,
            (TermRepr::Global(i), TermRepr::Global(j)) => i == j,
            (TermRepr::Univ(i), TermRepr::Univ(j)) => i.shallow_eq(j),
            (TermRepr::Lambda(a), TermRepr::Lambda(b))
            | (TermRepr::Forall(a), TermRepr::Forall(b))
            | (TermRepr::Application(a), TermRepr::Application(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }

    fn weaken_pair_by(
        &self,
        pair: &Arc<[Term; 2]>,
        var_arg: usize,
        var_body: usize,
        skipped_binders: usize,
    ) -> Arc<[Term; 2]> {
        let arg_ty = pair[0].weaken_by(var_arg, skipped_binders);
        let body = pair[1].weaken_by(var_body, skipped_binders);

        if pair[0].shallow_eq(&arg_ty) && pair[1].shallow_eq(&body) {
            pair.clone()
        } else {
            Arc::new([arg_ty, body])
        }
    }

    fn weaken_by(&self, var: usize, skipped_binders: usize) -> Self {
        let repr = match &self.repr {
            TermRepr::Var(i) => {
                if *i < var {
                    TermRepr::Var(*i)
                } else {
                    TermRepr::Var(i + skipped_binders)
                }
            }
            TermRepr::Global(ustr) => TermRepr::Global(*ustr),
            TermRepr::Univ(universe) => TermRepr::Univ(universe.clone()),
            TermRepr::Lambda(lam) => {
                let repr = self.weaken_pair_by(lam, var, var + 1, skipped_binders);
                TermRepr::Lambda(repr)
            }
            TermRepr::Forall(forall) => {
                let repr = self.weaken_pair_by(forall, var, var + 1, skipped_binders);
                TermRepr::Lambda(repr)
            }
            TermRepr::Application(app) => {
                let repr = self.weaken_pair_by(app, var, var, skipped_binders);
                TermRepr::Application(repr)
            }
        };

        Term { repr }
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

    pub fn type_of(&self, ctx: &mut Context) -> Result<Term, TypeError> {
        assert_eq!(ctx.types.len(), 0);
        let mut guard = DropGuard::new(ctx, |ctx| ctx.types.clear());

        self.type_(&mut guard)
    }

    pub fn type_(&self, ctx: &mut Context) -> Result<Term, TypeError> {
        match self.repr {
            TermRepr::Var(idx) => {
                let ty = ctx
                    .types
                    .get(ctx.types.len() - idx - 1)
                    .cloned()
                    .ok_or(TypeError::UnboundLocalVariable)?;

                if ty.is_univ() {
                    return Ok(ty);
                }

                Ok(ty.weaken_by(0, idx + 1))
            }
            TermRepr::Global(name) => ctx
                .globals
                .get(&name)
                .map(|x| x.ty().clone())
                .ok_or(TypeError::MissingGlobalVariable),
            TermRepr::Univ(ref universe) => Ok(Term {
                repr: TermRepr::Univ(universe.type_of()),
            }),
            TermRepr::Lambda(ref lam) => {
                let [arg_ty, body] = &**lam;
                let arg_sort = arg_ty.type_(ctx)?;

                if !arg_sort.is_univ() {
                    // arg_ty must be a type!
                    return Err(TypeError::ExpectedUniv { found: arg_sort });
                }

                ctx.types.push(arg_ty.clone());
                let body_ty = body.type_(ctx)?;
                ctx.types.pop();

                Ok(Term {
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
                    (repr, TermRepr::Univ(_)) | (_, repr) => {
                        return Err(TypeError::ExpectedUniv {
                            found: Term { repr },
                        });
                    }
                };

                Ok(Term {
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
                    repr => {
                        return Err(TypeError::ExpectedForall {
                            found: Term { repr },
                        });
                    }
                };

                if !arg_ty.is_defeq(&app_arg_ty, ctx) {
                    return Err(TypeError::ExpectedDefeq {
                        expected: arg_ty.clone(),
                        found: app_arg_ty,
                    });
                }

                Ok(ret_ty.subst(0, &app_arg_ty))
            }
        }
    }
}
