extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, Expr, ExprPath, ItemFn, Token};

#[proc_macro_attribute]
pub fn dexterous_developer_setup(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = parse_macro_input!(item as ItemFn);
    let vis = &ast.vis;

    let fn_name = &ast.sig.ident;

    let inner_fn_name_str = format!("dexterous_developered_inner_{fn_name}");
    let inner_fn_name = Ident::new(&inner_fn_name_str, Span::call_site());

    quote! {

        #[no_mangle]
        pub fn #inner_fn_name(app: &mut ReloadableAppContents) {
            #ast

            #fn_name(app);
        }

        #[allow(non_camel_case_types)]
        #vis struct #fn_name;

        impl dexterous_developer::ReloadableSetup for #fn_name {
            fn setup_function_name() -> &'static str {
                #inner_fn_name_str
            }

            fn default_function(app: &mut ReloadableAppContents) {
                #inner_fn_name(app);
            }
        }

    }
    .into()
}

#[proc_macro_attribute]
#[allow(clippy::needless_return)]
pub fn hot_bevy_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = parse_macro_input!(item as ItemFn);

    let fn_name: &proc_macro2::Ident = &ast.sig.ident;

    let mut stream: Vec<TokenStream> = vec![];
    #[cfg(feature = "hot")]
    {
        stream.push(
            quote! {
                #[allow(dead_code)]
                pub fn #fn_name(options: dexterous_developer::HotReloadOptions) {
                    dexterous_developer::run_reloadabe_app(options);
                }
            }
            .into(),
        );
    }
    #[cfg(feature = "hot_internal")]
    {
        stream.push(quote!{

                #[no_mangle]
                pub fn dexterous_developer_internal_main(library_paths: dexterous_developer::LibPathSet, initial_plugins: dexterous_developer::PluginSet) {
                    #ast

                    #fn_name(dexterous_developer::HotReloadPlugin::new(library_paths));
                }
            }.into());
    }
    #[cfg(not(any(feature = "hot", feature = "hot_internal")))]
    {
        stream.push(
            quote! {
                pub fn #fn_name(options: dexterous_developer::HotReloadOptions) {
                    #ast

                    #fn_name(dexterous_developer::InitialPluginsEmpty);
                }
            }
            .into(),
        );
    }
    TokenStream::from_iter(stream)
}

struct Loader {
    library: ExprPath,
    options: Expr,
}

impl Parse for Loader {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let items = Punctuated::<Expr, Token![,]>::parse_separated_nonempty(input).unwrap();
        let mut items = items.iter();
        let Some(library) = items.next().and_then(|a| match a {
            Expr::Path(p) => Some(p.clone()),
            _ => None,
        }) else {
            return Err(syn::Error::new(
                input.span(),
                "First item must be the path to your bevy main function",
            ));
        };
        let Some(options) = items.next().cloned() else {
            return Err(syn::Error::new(
                input.span(),
                "Second item must be an expression",
            ));
        };

        Ok(Self { library, options })
    }
}

#[proc_macro]
#[allow(clippy::needless_return, unused_variables, unreachable_code)]
pub fn hot_bevy_loader(args: TokenStream) -> TokenStream {
    let Loader { library, options } = parse_macro_input!(args as Loader);

    #[cfg(not(all(feature = "hot", feature = "hot_internal")))]
    {
        return quote! {
            #library(#options);
        }
        .into();
    }
    #[cfg(feature = "hot")]
    {
        return quote! {
            dexterous_developer::run_reloadabe_app(#options);
        }
        .into();
    }

    quote! {
        println!("This is a loader that shouldn't be called");
    }
    .into()
}
