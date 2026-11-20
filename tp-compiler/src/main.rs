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
    globals: ustr::UstrSet,
    locals_map: ustr::UstrMap<Vec<usize>>,
    locals: Vec<Ustr>,
    sort: ustr::UstrMap<u32>,
    level: ustr::UstrMap<u32>,
}

fn translate_doc(doc: &a::Document) -> k::Context {
    let mut ctx = k::Context::default();
    let mut nr = NameResolver::default();

    for global in &doc.globals {
        nr.clear();
        match global {
            a::Global::Definition(definition) => {
                ctx.add_definition(
                    definition.name.val,
                    nr.translate_term(0, &definition.expr),
                    nr.translate_term(0, &definition.ty),
                );
                nr.globals.insert(definition.name.val);
            }
            a::Global::Opaque(definition) => {
                ctx.add_opaque(
                    definition.name.val,
                    nr.translate_term(0, &definition.expr),
                    nr.translate_term(0, &definition.ty),
                );
                nr.globals.insert(definition.name.val);
            }
            a::Global::Axiom(axiom) => {
                ctx.add_axiom(axiom.name.val, nr.translate_term(0, &axiom.ty));
                nr.globals.insert(axiom.name.val);
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

    fn resolve(&self, ident: &a::Ident) -> Option<Result<usize, Ustr>> {
        self.resolve_local(ident).map(Ok).or_else(|| {
            if self.globals.contains(&ident.val) {
                Some(Err(ident.val))
            } else {
                None
            }
        })
    }

    fn resolve_sort(&mut self, ident: &a::Ident) -> u32 {
        let len = self.sort.len();
        *self
            .sort
            .entry(ident.val)
            .or_insert_with(|| len.try_into().unwrap())
    }

    fn resolve_level(&mut self, ident: &a::Ident) -> u32 {
        let len = self.level.len();
        *self
            .level
            .entry(ident.val)
            .or_insert_with(|| len.try_into().unwrap())
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
            tp_ast::Term::Variable(ident) => match self.resolve(ident) {
                Some(Ok(term)) => k::Term::var(depth - term),
                Some(Err(global)) => k::Term::global(global),
                None => panic!("unresolved ident: {}", ident.val),
            },
            tp_ast::Term::Univ(universe) => k::Term::univ(self.translate_univ(universe)),
            tp_ast::Term::Forall(data) => {
                let (arg_tys, mut body) = self.translate_closure(depth, data);

                for arg_ty in arg_tys.into_iter().rev() {
                    body = k::Term::forall(arg_ty, body)
                }

                body
            }
            tp_ast::Term::Lambda(data) => {
                let (arg_tys, mut body) = self.translate_closure(depth, data);

                for arg_ty in arg_tys.into_iter().rev() {
                    body = k::Term::lambda(arg_ty, body)
                }

                body
            }
            tp_ast::Term::Natural(_) => todo!(),
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
            tp_ast::UnpackedSort::SProp => k::Sort::sprop(),
            tp_ast::UnpackedSort::Prop => k::Sort::prop(),
            tp_ast::UnpackedSort::Type => k::Sort::ty(),
            tp_ast::UnpackedSort::Param(ident) => k::Sort::param(self.resolve_sort(&ident)),
            tp_ast::UnpackedSort::Trunc(sort) => self.translate_sort(sort).trunc(),
            tp_ast::UnpackedSort::Max(sorts) => {
                self.translate_max(sorts, Self::translate_sort, k::Sort::max)
            }
        }
    }

    fn translate_level(&mut self, level: &a::Level) -> k::Level {
        match level.unpack() {
            tp_ast::UnpackedLevel::Nat(offset) => k::Level::zero() + offset,
            tp_ast::UnpackedLevel::Param(ident, offset) => {
                k::Level::param(self.resolve_level(&ident)) + offset
            }
            tp_ast::UnpackedLevel::Add(level, offset) => self.translate_level(level) + offset,
            tp_ast::UnpackedLevel::When(sort, level) => {
                self.translate_level(level).when(self.translate_sort(sort))
            }
            tp_ast::UnpackedLevel::Max(levels) => {
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
