use proc_macro::TokenStream;

use syn::*;
use quote::{ToTokens, Tokens};

use std::str::FromStr;

/* 
Syntax

#[derive(HasItemKey)]
#[has_item_key(i32, expr="self.0")]
struct Tuple(i32)

#[derive(HasItemKey)]
#[has_item_key(i32, expr="self.id")]
struct S { id: i32 }

*/

struct Config {
  ty: Tokens,
  expr: Expr,
}

pub fn derive(input: TokenStream) -> TokenStream {
  let ast: DeriveInput = parse(input).unwrap();
  let ident = ast.ident;

  let configs: Vec<_> = ast
    .attrs
    .iter()
    .filter_map(|a| {
      a.path.segments.iter().next().and_then(|s| {
        if s.ident == "has_item_key" {
          a.interpret_meta().and_then(|meta| match meta {
            Meta::List(ref list) => {
              let args: Vec<_> = list.nested.iter().collect();
              if args.len() != 2 {
                panic!("#[has_item_key(..)] 2 arguments expected.")
              }

              let ty_tokens = match *args[0] {
                NestedMeta::Meta(Meta::Word(ref ident)) => ident.into_tokens(),
                NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                  ref ident,
                  lit: Lit::Str(ref lit),
                  ..
                })) if ident == "Key" =>
                {
                  let input = TokenStream::from_str(&lit.value()).unwrap();
                  let ty: Type = parse(input).unwrap();
                  ty.into_tokens()
                }
                _ => panic!("invalid type config."),
              };

              let expr_str = match *args[1] {
                NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                  ref ident,
                  lit: Lit::Str(ref lit),
                  ..
                })) if ident == "expr" =>
                {
                  lit.value()
                }
                NestedMeta::Literal(Lit::Str(ref lit)) => lit.value(),
                _ => panic!("invalid expr config,"),
              };

              Some(Config {
                ty: ty_tokens,
                expr: parse(TokenStream::from_str(&expr_str).unwrap()).unwrap(),
              })
            }
            _ => panic!("#[has_item_key(..)] invalid argument syntax."),
          })
        } else {
          None
        }
      })
    })
    .collect();

  if configs.is_empty() {
    panic!("No config attribute was found.");
  }

  let impls: Vec<_> = configs
    .into_iter()
    .map(|Config { ty, expr }| {
      quote! {
        impl ::s2_utils::list::HasItemKey<#ty> for #ident {
          fn get_item_key(&self) -> #ty {
            #expr
          }
        }
      }
    })
    .collect();
  let tokens = quote! {
    #(#impls)*
  };

  tokens.into()
}
