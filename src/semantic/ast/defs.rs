use std::collections::HashMap;

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
        &self,
        mut visit_duplicate: F,
    ) -> Vec<(Ident, RType<'ast>)> {
        let mut fields: HashMap<Symbol, (Ident, RType<'ast>)> =
            HashMap::with_capacity(self.fields.len());
        for v in &self.fields {
            match fields.get(&v.0) {
                Some(orig) => {
                    visit_duplicate(&orig.0, &v.0);
                }
                None => {
                    fields.insert(*v.0, *v);
                }
            }
        }
        fields.into_values().collect()
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
    pub fn unique_variants<F: FnMut(&Ident, &Ident)>(&self, mut visit_duplicate: F) -> Vec<Symbol> {
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
        variants.into_keys().collect()
    }
}

pub struct Function<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    // generics: Vec<Generic>
    pub args: Vec<(Ident, Type<'ast>)>,
    pub ret: Type<'ast>,
    pub body: Block<'ast>,
}
