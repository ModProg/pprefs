#![allow(dead_code)]

use std::fmt;

use proc_macro2::Span;
use quote::quote;
use syn::parse::{self, Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Colon, Comma, FatArrow, In};
use syn::{
    braced, parenthesized, Attribute, Error, Expr, ExprClosure, FnArg, Ident, Lit, LitStr,
    ReturnType, Type, Visibility,
};

mod kw {
    syn::custom_keyword!(sysfs_attr);
    syn::custom_keyword!(read);
    syn::custom_keyword!(write);
}

struct SysfsAttribute {
    span: Span,
    meta_attrs: Vec<Attribute>,
    fn_vis: Visibility,
    attr_name: Ident,
    attr_path_args: Punctuated<FnArg, Comma>,
    sysfs_dir: LitStr,
    getter: Option<GetterFunction>,
}

impl Parse for SysfsAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let meta_attrs = Attribute::parse_outer(input)?;
        let fn_vis = Visibility::parse(input)?;
        kw::sysfs_attr::parse(input)?;
        let attr_name = Ident::parse(input)?;
        let args;
        parenthesized!(args in input);
        let attr_path_args = args.parse_terminated(FnArg::parse, Comma)?;
        In::parse(input)?;
        let sysfs_dir = Lit::parse(input).and_then(|lit| match lit {
            Lit::Str(sysfs_path) => Ok(sysfs_path),
            _ => Err(Error::new(lit.span(), "expected a string literal")),
        })?;

        let braced;
        braced!(braced in input);

        let getter = if braced.peek(kw::read) {
            kw::read::parse(&braced)?;
            Colon::parse(&braced)?;
            let getter = braced.parse()?;
            Comma::parse(&braced)?;
            Some(getter)
        } else {
            None
        };

        Ok(Self {
            span: input.span(),
            meta_attrs,
            fn_vis,
            attr_name,
            attr_path_args,
            sysfs_dir,
            getter,
        })
    }
}

impl fmt::Debug for SysfsAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            meta_attrs,
            fn_vis,
            attr_name,
            attr_path_args,
            sysfs_dir,
            getter,
            ..
        } = self;
        let attr_path_args = attr_path_args.into_iter();
        write!(
            f,
            "{}",
            quote! {
                #(#meta_attrs)*
                #fn_vis sysfs_attr #attr_name(#(#attr_path_args)*) in #sysfs_dir {

                }
            }
        )
    }
}

struct GetterFunction {
    span: Span,
    parse_fn: ExprClosure,
    into_type: Box<Type>,
}

impl Parse for GetterFunction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        match &expr {
            Expr::Closure(
                parse_fn @ ExprClosure {
                    output: ReturnType::Type(_, ty),
                    ..
                },
            ) => Ok(Self {
                span: parse_fn.span(),
                parse_fn: parse_fn.clone(),
                into_type: ty.clone(),
            }),
            Expr::Closure(
                parse_fn @ ExprClosure {
                    output: ReturnType::Default,
                    ..
                },
            ) => {
                FatArrow::parse(input)?;
                Ok(Self {
                    span: parse_fn.span(),
                    parse_fn: parse_fn.clone(),
                    into_type: Box::new(Type::parse(input)?),
                })
            }
            _ => Err(Error::new(expr.span(), "expected a function closure")),
        }
    }
}

impl fmt::Debug for GetterFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            parse_fn,
            into_type,
            ..
        } = self;
        write!(
            f,
            "{}",
            quote!(GetterFunction {
                parse_fn: #parse_fn,
                into_type: #into_type,
            })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    macro_rules! test_parse {
        ({ $($input:tt)* } => $parse_ty:ty) => {{
            let result: syn::Result<$parse_ty> = syn::parse_str(&(quote::quote!{
                $($input)*
            }).to_string());
            match result {
                Ok(parsed) => dbg!(parsed),
                Err(e) => panic!("{}", e.to_string()),
            }
        }};
    }

    #[test]
    fn empty_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_useless_attr(item: usize) in "/fake/sysfs/path/item{item}" {}
        } => SysfsAttribute);
    }

    #[test]
    fn readonly_sysfs_attr_parses() {
        test_parse!({
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| text.parse().unwrap() => f32,
            }
        } => SysfsAttribute);
        test_parse!({
            pub sysfs_attr some_readonly_attr(item: usize) in "/fake/sysfs/path/item{item}" {
                read: |text| -> f32 { text.parse().unwrap() },
            }
        } => SysfsAttribute);
    }

    #[test]
    fn getter_closure_parses() {
        // With custom fat arrow return type syntax.
        test_parse!({
            |text| text.parse().unwrap() => isize
        } => GetterFunction);
        // With native Rust return type syntax.
        test_parse!({
            |text| -> isize { text.parse().unwrap() }
        } => GetterFunction);
    }
}
