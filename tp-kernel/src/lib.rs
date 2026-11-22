pub mod univ;

use std::{borrow::Cow, mem::DropGuard};

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
    Global(Arc<GlobalRef>),
    Univ(Universe),
    Lambda(Arc<[Term; 2]>),
    Forall(Arc<[Term; 2]>),
    Application(Arc<[Term; 2]>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlobalRef {
    name: Ustr,
    sorts: Box<[Sort]>,
    levels: Box<[Level]>,
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
    locals: Vec<Term>,
    globals: ustr::UstrMap<Global>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct UniverseParams {
    sort_vars: u16,
    level_vars: u16,
}

#[derive(Debug)]
enum Global {
    Definition {
        value: Term,
        ty: Term,
        params: UniverseParams,
    },
    Opaque {
        value: Term,
        ty: Term,
        params: UniverseParams,
    },
    Axiom {
        ty: Term,
        params: UniverseParams,
    },
}

#[derive(Debug)]
pub enum TypeError {
    UnboundLocalVariable,
    ExpectedUniv { found: Term },
    MissingGlobalVariable,
    ExpectedForall { found: Term },
    ExpectedDefeq { expected: Term, found: Term },
}

impl Context {
    pub fn add_definition(&mut self, name: Ustr, term: Term, ty: Term) {
        assert!(self.locals.is_empty());
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
                let params = term.universe_params().max(ty.universe_params());

                entry.insert(Global::Definition {
                    value: term,
                    ty,
                    params,
                });
            }
        }
    }

    pub fn add_opaque(&mut self, name: Ustr, term: Term, ty: Term) {
        assert!(self.locals.is_empty());
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
                let params = term.universe_params().max(ty.universe_params());

                entry.insert(Global::Opaque {
                    value: term,
                    ty,
                    params,
                });
            }
        }
    }

    pub fn add_axiom(&mut self, name: Ustr, ty: Term) {
        assert!(self.locals.is_empty());
        if !ty.type_of(self).is_ok_and(|t| t.is_univ()) {
            panic!("expected a type but found: {ty:?}")
        }

        match self.globals.entry(name) {
            std::collections::hash_map::Entry::Occupied(_) => {
                panic!("duplicate global name")
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let params = ty.universe_params();

                entry.insert(Global::Axiom { ty, params });
            }
        }
    }
}

impl Global {
    fn ty(&self) -> &Term {
        match self {
            Global::Definition {
                value: _,
                ty,
                params: _,
            }
            | Global::Opaque {
                value: _,
                ty,
                params: _,
            }
            | Global::Axiom { ty, params: _ } => ty,
        }
    }

    fn params(&self) -> UniverseParams {
        match self {
            Global::Definition {
                value: _,
                ty: _,
                params,
            }
            | Global::Opaque {
                value: _,
                ty: _,
                params,
            }
            | Global::Axiom { ty: _, params } => *params,
        }
    }
}

impl Term {
    pub fn var(id: usize) -> Self {
        Self {
            repr: TermRepr::Var(id),
        }
    }

    pub fn global(name: Ustr, sorts: Box<[Sort]>, levels: Box<[Level]>) -> Self {
        Self {
            repr: TermRepr::Global(Arc::new(GlobalRef {
                name,
                sorts,
                levels,
            })),
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

    fn merge_pair_by(pair: &Arc<[Term; 2]>, a: Term, b: Term) -> Arc<[Term; 2]> {
        if pair[0].shallow_eq(&a) && pair[1].shallow_eq(&b) {
            pair.clone()
        } else {
            Arc::new([a, b])
        }
    }

    fn weaken_pair_by(
        pair: &Arc<[Term; 2]>,
        var_arg: usize,
        var_body: usize,
        skipped_binders: usize,
    ) -> Arc<[Term; 2]> {
        let arg_ty = pair[0].weaken_by(var_arg, skipped_binders);
        let body = pair[1].weaken_by(var_body, skipped_binders);
        Self::merge_pair_by(pair, arg_ty, body)
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
            TermRepr::Global(ustr) => TermRepr::Global(ustr.clone()),
            TermRepr::Univ(universe) => TermRepr::Univ(universe.clone()),
            TermRepr::Lambda(lam) => {
                let repr = Self::weaken_pair_by(lam, var, var + 1, skipped_binders);
                TermRepr::Lambda(repr)
            }
            TermRepr::Forall(forall) => {
                let repr = Self::weaken_pair_by(forall, var, var + 1, skipped_binders);
                TermRepr::Lambda(repr)
            }
            TermRepr::Application(app) => {
                let repr = Self::weaken_pair_by(app, var, var, skipped_binders);
                TermRepr::Application(repr)
            }
        };

        Term { repr }
    }

    fn subst_pair_by(
        pair: &Arc<[Term; 2]>,
        var_arg: usize,
        var_body: usize,
        subst: &Self,
        skipped_binders: usize,
    ) -> Arc<[Term; 2]> {
        let arg_ty = pair[0].subst_(var_arg, subst, skipped_binders);
        let body = pair[1].subst_(var_body, subst, skipped_binders);
        Self::merge_pair_by(pair, arg_ty, body)
    }

    fn subst_(&self, var: usize, subst: &Self, skipped_binders: usize) -> Self {
        let repr = match &self.repr {
            TermRepr::Var(i) => match var.cmp(i) {
                std::cmp::Ordering::Less => TermRepr::Var(*i - 1),
                std::cmp::Ordering::Greater => TermRepr::Var(*i),
                std::cmp::Ordering::Equal => return subst.weaken_by(0, skipped_binders),
            },
            TermRepr::Global(name) => TermRepr::Global(name.clone()),
            TermRepr::Univ(universe) => TermRepr::Univ(universe.clone()),
            TermRepr::Lambda(lam) => TermRepr::Lambda(Self::subst_pair_by(
                lam,
                var,
                var + 1,
                subst,
                skipped_binders,
            )),
            TermRepr::Forall(forall) => TermRepr::Forall(Self::subst_pair_by(
                forall,
                var,
                var + 1,
                subst,
                skipped_binders,
            )),
            TermRepr::Application(app) => {
                TermRepr::Forall(Self::subst_pair_by(app, var, var, subst, skipped_binders))
            }
        };

        Term { repr }
    }

    fn subst(&self, var: usize, subst: &Self, skipped_binders: usize) -> Self {
        self.subst_(var, subst, skipped_binders)
    }

    fn is_defeq(&self, other: &Self, ctx: &mut Context) -> bool {
        // let len = ctx.types.len();
        // let _guard = DropGuard::new(ctx, |ctx| ctx.types.truncate(len));

        if self == other {
            return true;
        }

        let a = self.normalize(ctx);
        let b = other.normalize(ctx);

        matches!((a, b), (Ok(a), Ok(b)) if a == b)
    }

    pub fn normalize(&self, ctx: &Context) -> Result<Self, TypeError> {
        let mut this = Cow::Borrowed(self);

        let repr = loop {
            match &this.repr {
                TermRepr::Var(i) => break TermRepr::Var(*i),
                TermRepr::Global(ustr) => {
                    return ctx
                        .globals
                        .get(&ustr.name)
                        .map(|ty| match ty {
                            Global::Definition {
                                value: def,
                                ty: _,
                                params: _,
                            } => def.clone(),
                            Global::Opaque { .. } | Global::Axiom { .. } => this.into_owned(),
                        })
                        .ok_or(TypeError::MissingGlobalVariable);
                }
                TermRepr::Univ(_) => return Ok(this.into_owned()),
                TermRepr::Lambda(lam) => {
                    let arg_ty = lam[0].normalize(ctx)?;
                    let body = lam[1].normalize(ctx)?;
                    break TermRepr::Lambda(Self::merge_pair_by(lam, arg_ty, body));
                }
                TermRepr::Forall(forall) => {
                    let arg_ty = forall[0].normalize(ctx)?;
                    let body = forall[1].normalize(ctx)?;
                    break TermRepr::Forall(Self::merge_pair_by(forall, arg_ty, body));
                }
                TermRepr::Application(app) => {
                    let arg_ty = app[0].normalize(ctx)?;
                    let body = app[1].normalize(ctx)?;

                    match arg_ty.repr {
                        TermRepr::Lambda(a) | TermRepr::Forall(a) => {
                            this = Cow::Owned(a[1].subst(0, &body, 0));
                            continue;
                        }
                        TermRepr::Var(_)
                        | TermRepr::Global(_)
                        | TermRepr::Univ(_)
                        | TermRepr::Application(_) => (),
                    }

                    break TermRepr::Application(Self::merge_pair_by(app, arg_ty, body));
                }
            }
        };

        Ok(Self { repr })
    }

    pub fn type_of(&self, ctx: &mut Context) -> Result<Term, TypeError> {
        assert_eq!(ctx.locals.len(), 0);
        let mut guard = DropGuard::new(ctx, |ctx| ctx.locals.clear());

        self.type_(&mut guard)
    }

    pub fn type_(&self, ctx: &mut Context) -> Result<Term, TypeError> {
        match self.repr {
            TermRepr::Var(idx) => {
                let ty = ctx
                    .locals
                    .get(ctx.locals.len() - idx - 1)
                    .cloned()
                    .ok_or(TypeError::UnboundLocalVariable)?;

                if ty.is_univ() {
                    return Ok(ty);
                }

                Ok(ty.weaken_by(0, idx + 1))
            }
            TermRepr::Global(ref g) => ctx
                .globals
                .get(&g.name)
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

                ctx.locals.push(arg_ty.clone());
                let body_ty = body.type_(ctx)?;
                ctx.locals.pop();

                Ok(Term {
                    repr: TermRepr::Forall(Arc::new([arg_ty.clone(), body_ty])),
                })
            }
            TermRepr::Forall(ref forall) => {
                let [arg_ty, body_ty] = &**forall;

                let arg_sort = arg_ty.type_(ctx)?;
                ctx.locals.push(arg_ty.clone());
                let body_sort = body_ty.type_(ctx)?;
                ctx.locals.pop();

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

                Ok(ret_ty.subst(0, &app_arg_ty, 0))
            }
        }
    }

    fn universe_params(&self) -> UniverseParams {
        match &self.repr {
            TermRepr::Var(_) => Default::default(),
            TermRepr::Global(g) => g
                .sorts
                .iter()
                .map(Sort::universe_params)
                .chain(g.levels.iter().map(Level::universe_params))
                .collect(),
            TermRepr::Univ(universe) => universe
                .sort
                .universe_params()
                .max(universe.level.universe_params()),
            TermRepr::Lambda(t) | TermRepr::Forall(t) | TermRepr::Application(t) => {
                t[0].universe_params().max(t[1].universe_params())
            }
        }
    }
}

impl UniverseParams {
    fn max(self, other: Self) -> Self {
        Self {
            sort_vars: self.sort_vars.max(other.sort_vars),
            level_vars: self.level_vars.max(other.level_vars),
        }
    }
}

impl FromIterator<Self> for UniverseParams {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        iter.into_iter().fold(Default::default(), Self::max)
    }
}
