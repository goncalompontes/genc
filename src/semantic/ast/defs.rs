use std::collections::HashMap;

use crate::semantic::ast::RBlock;

use super::{Block, Ident, RDataDef, REnumDef, RType, Symbol, Type, Visibility};

pub enum TypeDef<'ast> {
    Enum(REnumDef<'ast>),
    Data(RDataDef<'ast>),
}

impl<'ast> TypeDef<'ast> {
    pub fn vis(&self) -> Visibility {
        match self {
            TypeDef::Enum(enum_def) => enum_def.vis,
            TypeDef::Data(data_def) => data_def.vis,
        }
    }

    pub fn name(&self) -> &Ident {
        match self {
            TypeDef::Enum(enum_def) => &enum_def.name,
            TypeDef::Data(data_def) => &data_def.name,
        }
    }
}

pub struct Alias<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    pub ty: RType<'ast>,
}

// We will keep structs as simple structs for now
pub struct DataDef<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    pub fields: Vec<(Ident, RType<'ast>)>,
}

impl<'ast> DataDef<'ast> {
    pub fn unique_fields<F: FnMut(&Ident, &Ident)>(
        &'ast self,
        mut visit_duplicate: F,
    ) -> impl Iterator<Item = (Ident, &'ast RType<'ast>)> {
        let mut fields: HashMap<Symbol, (Ident, &'_ RType<'ast>)> =
            HashMap::with_capacity(self.fields.len());
        for v in self.fields.iter() {
            match fields.get(&v.0) {
                Some(orig) => {
                    visit_duplicate(&orig.0, &v.0);
                }
                None => {
                    fields.insert(*v.0, (v.0, &v.1));
                }
            }
        }
        fields.into_values()
    }
}

// We will keep enums as simple enums for now
pub struct EnumDef<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    pub variants: Vec<Ident>,
    _marker: std::marker::PhantomData<&'ast ()>,
}

impl<'ast> EnumDef<'ast> {
    pub fn unique_variants<F: FnMut(&Ident, &Ident)>(
        &self,
        mut visit_duplicate: F,
    ) -> impl Iterator<Item = Symbol> {
        let mut variants: HashMap<Symbol, Ident> = HashMap::with_capacity(self.variants.len());
        for v in &self.variants {
            match variants.get(v) {
                Some(orig) => {
                    visit_duplicate(orig, v);
                }
                None => {
                    variants.insert(**v, *v);
                }
            }
        }
        variants.into_keys()
    }
}

pub struct Function<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    // generics: Vec<Generic>
    pub args: Vec<(Ident, RType<'ast>)>,
    pub ret: RType<'ast>,
    pub body: RBlock<'ast>,
}

impl<'ast> Function<'ast> {
    pub fn unique_args<F: FnMut(&Ident, &Ident)>(
        &self,
        mut visit_duplicate: F,
    ) -> impl Iterator<Item = (Ident, RType<'ast>)> {
        let mut fields: HashMap<Symbol, (Ident, RType<'ast>)> =
            HashMap::with_capacity(self.args.len());
        for v in &self.args {
            match fields.get(&v.0) {
                Some(orig) => {
                    visit_duplicate(&orig.0, &v.0);
                }
                None => {
                    fields.insert(*v.0, *v);
                }
            }
        }
        fields.into_values()
    }
}
