extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, ItemFn, Path};

#[proc_macro_attribute]
pub fn dexterous_developer_setup(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = parse_macro_input!(item as ItemFn);
    let vis = &ast.vis;

    let fn_name = &ast.sig.ident;
    let path = if attr.is_empty() {
        "".to_string()
    } else {
        parse_macro_input!(attr as Path)
            .segments
            .iter()
            .map(|v| v.ident.to_string())
            .collect::<Vec<_>>()
            .join("_")
    };

    let inner_fn_name_str = format!("{path}_dexterous_developered_inner_{fn_name}");
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
    #[cfg(feature = "hot_internal")]
    {
        stream.push(quote!{

                #[no_mangle]
                pub extern "system" fn dexterous_developer_internal_main(library_paths: std::ffi::CString, closure: fn() -> ()) {
                    #ast
                    #fn_name(dexterous_developer::bevy_support::HotReloadPlugin::new(library_paths, closure));
                }
            }.into());
    }
    stream.push(
        quote! {
            pub fn #fn_name() {
                #ast

                #fn_name(dexterous_developer::InitialPluginsEmpty);
            }
        }
        .into(),
    );

    TokenStream::from_iter(stream)
}
