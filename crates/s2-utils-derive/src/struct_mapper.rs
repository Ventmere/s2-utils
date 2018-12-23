use quote::{self, ToTokens};
use syn::*;
use proc_macro::TokenStream;

struct FromStruct {
  from_ty: quote::Tokens,
  from_ty_has_lifetime: bool,
  from_expr: quote::Tokens,
  fields: Vec<FieldConfig>,
}

struct FieldConfig {
  name: Ident,
  borrow: bool,
  expr: Box<quote::ToTokens>,
}

pub fn derive(input: TokenStream) -> TokenStream {
  let ast: DeriveInput = parse(input).unwrap();
  let name = &ast.ident;

  if ast.generics.lifetimes().count() > 1 {
    panic!("#[derive(StructMapper)] only supports 1 lifetime param")
  }

  let lifetime: Option<&LifetimeDef> = { ast.generics.lifetimes().next() };

  let fields: Vec<&Field> = match ast.data {
    Data::Struct(DataStruct {
      fields: Fields::Named(FieldsNamed { ref named, .. }),
      ..
    }) => named.iter().collect(),
    _ => panic!("#[derive(StructMapper)] only supports non-unit struct with named fields"),
  };

  let mut from_structs = vec![];

  for attr in &ast.attrs {
    let meta = attr.interpret_meta().expect("parse attr meta");
    match meta {
      Meta::List(MetaList {
        ref ident,
        ref nested,
        ..
      }) => match ident.as_ref() {
        "map_from" => {
          from_structs.push(parse_map_from(nested.iter().collect()));
        }
        _ => {}
      },
      _ => {}
    };
  }

  for s in &mut from_structs {
    let from_expr = &s.from_expr;
    let unmapped_fields: Vec<(&Ident, &Type)> = fields
      .iter()
      .filter_map(|f| {
        if s.fields.iter().all(|sf| Some(sf.name) != f.ident) {
          Some((f.ident.as_ref().expect("unmapped field ident"), &f.ty))
        } else {
          None
        }
      })
      .collect();

    for (ident, ty) in unmapped_fields {
      s.fields.push(FieldConfig {
        name: ident.clone(),
        borrow: match *ty {
          Type::Reference(_) => true,
          _ => false,
        },
        expr: Box::new(quote! { #from_expr.#ident }),
      });
    }
  }

  let items: Vec<_> = from_structs
    .into_iter()
    .map(|s| {
      let from_ty = s.from_ty;
      let fields: Vec<_> = s.fields
        .into_iter()
        .map(|f| {
          let name = f.name;
          let expr = f.expr;
          if f.borrow {
            quote! {
              #name: &#expr
            }
          } else {
            quote! {
              #name: #expr
            }
          }
        })
        .collect();

      match (lifetime, s.from_ty_has_lifetime) {
        (Some(_), false) => {
          quote! {
            impl<'a, 'b: 'a> From<&'b #from_ty> for #name<'a> {
              fn from(from: &'b #from_ty) -> Self {
                #name {
                  #(#fields),*
                }
              }
            }
          }
        }
        (Some(_), true) => {
          quote! {
            impl<'a> From<#from_ty> for #name<'a> {
              fn from(from: #from_ty) -> Self {
                #name {
                  #(#fields),*
                }
              }
            }
          }
        }
        (None, true) => {
          quote! {
            impl<'a> From<#from_ty> for #name {
              fn from(from: #from_ty) -> Self {
                #name {
                  #(#fields),*
                }
              }
            }
          }
        }
        _ => {
          quote! {
            impl From<#from_ty> for #name {
              fn from(from: #from_ty) -> Self {
                #name {
                  #(#fields),*
                }
              }
            }
          }
        }
      }
    })
    .collect();

  let code = quote! {
    #(#items)*
  };

  code.into()
}

fn parse_map_from(items: Vec<&NestedMeta>) -> FromStruct {
  if items.is_empty() {
    panic!("#[map_from(...)] expects at least 1 item");
  }

  let ty_item = items[0];
  let (from_ty, has_lifetime) = match *ty_item {
    NestedMeta::Meta(Meta::Word(ref ident)) => (quote!{ #ident }, false),
    NestedMeta::Meta(Meta::NameValue(MetaNameValue {
      ref ident,
      lit: Lit::Str(ref ty_lit),
      ..
    })) if ident == "From" =>
    {
      let ty_str = ty_lit.value();
      //TODO parse, not guess
      let lifetime_count = ty_str.chars().filter(|c| c == &'\'').count();
      let ty: Type = parse_str(&ty_str).unwrap();

      if lifetime_count > 1 {
        panic!("#[map_from(...)] from type has more than 1 lifetime");
      }

      (ty.into_tokens(), lifetime_count > 0)
    }
    _ => {
      panic!("#[map_from(...)] first item should be a Type or 'From=\"Type\"'");
    }
  };

  let mut v = FromStruct {
    from_ty: from_ty,
    from_ty_has_lifetime: has_lifetime,
    from_expr: quote! { from },
    fields: vec![],
  };

  for item in items.iter().skip(1) {
    match **item {
      NestedMeta::Meta(Meta::List(MetaList {
        ref ident,
        ref nested,
        ..
      })) if ident == "fields" =>
      {
        for item in nested.iter() {
          v.fields.push(parse_field_config(item));
        }
      }
      NestedMeta::Meta(Meta::NameValue(MetaNameValue {
        ref ident,
        lit: Lit::Str(ref lit),
        ..
      })) => match ident.as_ref() {
        "from" => {
          let expr = parse_expr(&lit.value());
          v.from_expr = expr.into_tokens();
        }
        _ => {
          panic!("#[map_from(...)] unknown item '{}'", ident);
        }
      },
      _ => {}
    }
  }

  v
}

fn parse_field_config(item: &NestedMeta) -> FieldConfig {
  match *item {
    // use default value
    NestedMeta::Meta(Meta::Word(ref ident)) => {
      FieldConfig {
        name: ident.clone(),
        borrow: false,
        expr: Box::new(quote! { Default::default() }),
      }
    },
    // use computed value
    NestedMeta::Meta(Meta::NameValue(MetaNameValue { ref ident, lit: Lit::Str(ref lit), .. })) => {
      FieldConfig {
        name: ident.clone(),
        borrow: false,
        expr: Box::new(parse_expr(&lit.value())),
      }
    },
    ref rest => {
      panic!("#[map_from(T, fields(...)] unexpected field config item '{}'. expecting 'ident' or 'ident = \"expr\"'", quote! { #rest })
    },
  }
}

fn parse_expr(code: &str) -> Expr {
  match parse_str(code) {
    Ok(expr) => expr,
    Err(err) => panic!("{}: {}", err, code),
  }
}
