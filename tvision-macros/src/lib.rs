use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Ident, ImplItem, ItemImpl, Token};

/// `#[delegate(to = <field>)]` — inject forwarders for un-provided trait methods.
#[proc_macro_attribute]
pub fn delegate(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as DelegateArgs);
    let item_impl = syn::parse_macro_input!(item as ItemImpl);
    match expand(args, item_impl) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct DelegateArgs {
    field: Ident,
    skip: Vec<Ident>,
}

impl syn::parse::Parse for DelegateArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut field = None;
        let mut skip = Vec::new();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "to" {
                input.parse::<Token![=]>()?;
                field = Some(input.parse()?);
            } else if key == "skip" {
                let content;
                syn::parenthesized!(content in input);
                let names = content.parse_terminated(Ident::parse, Token![,])?;
                skip.extend(names);
            } else {
                return Err(syn::Error::new(key.span(), "expected `to` or `skip`"));
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        let field = field.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "#[delegate]: missing `to = <field>`")
        })?;
        Ok(DelegateArgs { field, skip })
    }
}

/// Resolve the path prefix for crate `tvision` that is valid at the call site
/// (inside the lib via `extern crate self`, in examples, and downstream under
/// ANY alias the consumer chooses). ALWAYS returns an `::<ident>` form — never `crate`.
fn tvision_path() -> TokenStream2 {
    use proc_macro_crate::{FoundCrate, crate_name};
    let ident = match crate_name("tvision") {
        // `Itself` happens when compiling the tvision lib AND its own examples;
        // `extern crate self as tvision;` makes `::tvision` valid in the lib,
        // and the example's implicit dep makes `::tvision` valid there too.
        Ok(FoundCrate::Itself) => Ident::new("tvision", Span::call_site()),
        Ok(FoundCrate::Name(name)) => Ident::new(&name, Span::call_site()),
        // Fall back to the canonical name; a wrong name yields a clear
        // unresolved-path error rather than a silently wrong expansion.
        Err(_) => Ident::new("tvision", Span::call_site()),
    };
    quote! { ::#ident }
}

fn expand(args: DelegateArgs, mut item_impl: ItemImpl) -> syn::Result<TokenStream2> {
    let trait_path = item_impl.trait_.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(
            &item_impl.self_ty,
            "#[delegate] must be placed on an `impl Trait for Type` block",
        )
    })?;
    let trait_ident = trait_path.1.segments.last().unwrap().ident.to_string();

    let provided: std::collections::HashSet<String> = item_impl
        .items
        .iter()
        .filter_map(|it| match it {
            ImplItem::Fn(f) => Some(f.sig.ident.to_string()),
            _ => None,
        })
        .collect();
    let skip: std::collections::HashSet<String> = args.skip.iter().map(|i| i.to_string()).collect();

    let krate = tvision_path();
    let field = &args.field;

    // SPIKE: only `View::cursor_request` is known (exercises `#krate::Point` resolution).
    // (More methods land in a later task.)
    if trait_ident != "View" {
        return Err(syn::Error::new(
            Span::call_site(),
            format!("#[delegate]: unknown delegatable trait `{trait_ident}`"),
        ));
    }
    let candidates: Vec<(&str, TokenStream2)> = vec![(
        "cursor_request",
        quote! {
            fn cursor_request(&self) -> ::core::option::Option<#krate::Point> {
                self.#field.cursor_request()
            }
        },
    )];

    for (name, tokens) in candidates {
        if !provided.contains(name) && !skip.contains(name) {
            let f: ImplItem = syn::parse2(tokens)?;
            item_impl.items.push(f);
        }
    }
    Ok(quote! { #item_impl })
}
