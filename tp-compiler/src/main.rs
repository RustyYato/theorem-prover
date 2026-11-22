use std::path::PathBuf;

use tp_ast as a;
use tp_kernel as k;
use ustr::Ustr;

#[derive(clap::Parser)]
struct Args {
    path: PathBuf,
}

fn main() -> eyre::Result {
    color_eyre::install().unwrap();
    let args: Args = clap::Parser::parse();
    let file = fs_err::read(args.path)?;
    let parser = tp_parser::Parser::new(&file);
    let (doc, _newlines) = parser.parse_file();
    dbg!(&doc);
    let ctx = translate_doc(&doc);
    dbg!(ctx);
    Ok(())
}

#[derive(Default)]
struct NameResolver {
    globals: ustr::UstrMap<(u16, u16)>,
    locals_map: ustr::UstrMap<Vec<usize>>,
    locals: Vec<Ustr>,
    sort: ustr::UstrMap<u16>,
    level: ustr::UstrMap<u16>,
}

fn translate_doc(doc: &a::Document) -> k::Context {
    let mut ctx = k::Context::default();
    let mut nr = NameResolver::default();

    for global in &doc.globals {
        nr.clear();
        match global {
            a::Global::Definition(definition) => {
                let sorts = definition.sorts.len().try_into().unwrap();
                let levels = definition.levels.len().try_into().unwrap();

                for (i, sort) in definition.sorts.iter().enumerate() {
                    nr.sort.insert(*sort, i as _);
                }
                for (i, sort) in definition.levels.iter().enumerate() {
                    nr.level.insert(*sort, i as _);
                }

                ctx.add_definition(
                    definition.name.val,
                    nr.translate_term(0, &definition.expr),
                    nr.translate_term(0, &definition.ty),
                );
                nr.globals.insert(definition.name.val, (sorts, levels));
            }
            a::Global::Opaque(definition) => {
                let sorts = definition.sorts.len().try_into().unwrap();
                let levels = definition.levels.len().try_into().unwrap();

                for (i, sort) in definition.sorts.iter().enumerate() {
                    nr.sort.insert(*sort, i as _);
                }
                for (i, sort) in definition.levels.iter().enumerate() {
                    nr.level.insert(*sort, i as _);
                }

                ctx.add_opaque(
                    definition.name.val,
                    nr.translate_term(0, &definition.expr),
                    nr.translate_term(0, &definition.ty),
                );
                nr.globals.insert(definition.name.val, (sorts, levels));
            }
            a::Global::Axiom(axiom) => {
                let sorts = axiom.sorts.len().try_into().unwrap();
                let levels = axiom.levels.len().try_into().unwrap();

                for (i, sort) in axiom.sorts.iter().enumerate() {
                    nr.sort.insert(*sort, i as _);
                }
                for (i, sort) in axiom.levels.iter().enumerate() {
                    nr.level.insert(*sort, i as _);
                }

                ctx.add_axiom(axiom.name.val, nr.translate_term(0, &axiom.ty));
                nr.globals.insert(axiom.name.val, (sorts, levels));
            }
        }
    }

    ctx
}

impl NameResolver {
    fn resolve_local(&self, ident: &a::Ident) -> Option<usize> {
        match self.locals_map.get(&ident.val) {
            Some(terms) => {
                if let [.., term] = *terms.as_slice() {
                    Some(term)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn resolve(&self, ident: &a::Ident) -> Option<Result<usize, (Ustr, u16, u16)>> {
        self.resolve_local(ident).map(Ok).or_else(|| {
            if let Some((s, l)) = self.resolve_global(ident) {
                Some(Err((ident.val, s, l)))
            } else {
                None
            }
        })
    }

    fn resolve_global(&self, ident: &a::Ident) -> Option<(u16, u16)> {
        self.globals.get(&ident.val).copied()
    }

    fn resolve_sort(&mut self, ident: &a::Ident) -> u16 {
        *self.sort.get(&ident.val).unwrap()
    }

    fn resolve_level(&mut self, ident: &a::Ident) -> u16 {
        *self.level.get(&ident.val).unwrap()
    }

    fn exit_scope(&mut self, mark: usize) {
        for i in self.locals.drain(mark..) {
            if let Some(names) = self.locals_map.get_mut(&i) {
                names.pop();
            }
        }
    }

    fn translate_argument(&mut self, depth: usize, arg: &a::Argument) -> k::Term {
        let term = self.translate_term(depth, &arg.ty);
        let names_map = self.locals_map.entry(arg.name.val).or_default();
        names_map.push(depth + 1);
        self.locals.push(arg.name.val);
        term
    }

    fn translate_closure(
        &mut self,
        mut depth: usize,
        data: &a::SliceWithHeader<a::Term, a::Argument>,
    ) -> (Vec<k::Term>, k::Term) {
        depth.checked_add(data.slice.len()).unwrap();

        let mark = self.locals.len();

        let mut arg_tys = Vec::with_capacity(data.slice.len());
        for arg in &data.slice {
            let arg_ty = self.translate_argument(depth, arg);
            arg_tys.push(arg_ty);
            depth += 1;
        }

        let body = self.translate_term(depth, &data.header);

        self.exit_scope(mark);

        (arg_tys, body)
    }

    fn translate_term(&mut self, depth: usize, term: &a::Term) -> k::Term {
        match term {
            a::Term::Variable(ident) => match self.resolve(ident) {
                Some(Ok(term)) => k::Term::var(depth - term),
                Some(Err((name, 0, 0))) => {
                    k::Term::global(name, Default::default(), Default::default())
                }
                Some(Err((name, _, _))) => {
                    panic!(
                        "Tried to reference global ({name}) with sort or level parameters without supplying parameters"
                    )
                }
                None => panic!("unresolved ident: {}", ident.val),
            },
            a::Term::GlobalRef(g) => match self.resolve_global(&g.ident) {
                Some((s, l)) => {
                    let name = g.ident.val;
                    if g.sorts.len() != s as usize {
                        panic!(
                            "Tried to reference global ({name}) with the wrong number of sort parameters"
                        )
                    }

                    if g.levels.len() != l as usize {
                        panic!(
                            "Tried to reference global ({name}) with the wrong number of level parameters"
                        )
                    }

                    k::Term::global(
                        name,
                        g.sorts.iter().map(|s| self.translate_sort(s)).collect(),
                        g.levels.iter().map(|l| self.translate_level(l)).collect(),
                    )
                }
                None => panic!("unresolved global ident: {}", g.ident.val),
            },
            a::Term::Univ(universe) => k::Term::univ(self.translate_univ(universe)),
            a::Term::Forall(data) => {
                let (arg_tys, mut body) = self.translate_closure(depth, data);

                for arg_ty in arg_tys.into_iter().rev() {
                    body = k::Term::forall(arg_ty, body)
                }

                body
            }
            a::Term::Lambda(data) => {
                let (arg_tys, mut body) = self.translate_closure(depth, data);

                for arg_ty in arg_tys.into_iter().rev() {
                    body = k::Term::lambda(arg_ty, body)
                }

                body
            }
            a::Term::Application(app) => {
                let mut term = self.translate_term(depth, &app.header);

                for arg in &app.slice {
                    let arg = self.translate_term(depth, arg);
                    term = term.apply(arg);
                }

                term
            }
            a::Term::Natural(_) => todo!(),
        }
    }

    fn translate_univ(&mut self, universe: &a::Universe) -> k::Universe {
        k::Universe {
            sort: self.translate_sort(&universe.sort),
            level: self.translate_level(&universe.level),
        }
    }

    fn translate_max<F, G, T, U>(&mut self, data: &[T], f: F, g: G) -> U
    where
        F: Copy + Fn(&mut Self, &T) -> U,
        G: Copy + Fn(U, U) -> U,
    {
        match data {
            [] => unreachable!(),
            [sort] => f(self, sort),
            _ => {
                let (l, r) = data.split_at(data.len() / 2);
                let l = self.translate_max(l, f, g);
                let r = self.translate_max(r, f, g);
                g(l, r)
            }
        }
    }

    fn translate_sort(&mut self, sort: &a::Sort) -> k::Sort {
        match sort.unpack() {
            a::UnpackedSort::SProp => k::Sort::sprop(),
            a::UnpackedSort::Prop => k::Sort::prop(),
            a::UnpackedSort::Type => k::Sort::ty(),
            a::UnpackedSort::Param(ident) => k::Sort::param(self.resolve_sort(&ident)),
            a::UnpackedSort::Trunc(sort) => self.translate_sort(sort).trunc(),
            a::UnpackedSort::Max(sorts) => {
                self.translate_max(sorts, Self::translate_sort, k::Sort::max)
            }
        }
    }

    fn translate_level(&mut self, level: &a::Level) -> k::Level {
        match level.unpack() {
            a::UnpackedLevel::Nat(offset) => k::Level::zero() + offset,
            a::UnpackedLevel::Param(ident, offset) => {
                k::Level::param(self.resolve_level(&ident)) + offset
            }
            a::UnpackedLevel::Add(level, offset) => self.translate_level(level) + offset,
            a::UnpackedLevel::When(sort, level) => {
                self.translate_level(level).when(self.translate_sort(sort))
            }
            a::UnpackedLevel::Max(levels) => {
                self.translate_max(levels, Self::translate_level, k::Level::max)
            }
        }
    }

    fn clear(&mut self) {
        let Self {
            globals: _,
            locals_map,
            locals,
            level,
            sort,
        } = self;

        level.clear();
        sort.clear();
        locals.clear();
        locals_map.clear();
    }
}
