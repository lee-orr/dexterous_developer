extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn dexterous_developer_setup(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = parse_macro_input!(item as ItemFn);

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
        struct #fn_name;

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

    #[cfg(feature = "hot")]
    {
        return quote! {
            pub fn #fn_name(options: dexterous_developer::HotReloadOptions) {
                dexterous_developer::run_reloadabe_app(options);
            }

            #[no_mangle]
            pub fn dexterous_developer_internal_main(plugin: dexterous_developer::HotReloadPlugin) {
                #ast

                let initial = dexterous_developer::InitialPlugins::new(plugin);

                #fn_name(initial);
            }
        }
        .into();
    }

    #[cfg(not(feature = "hot"))]
    {
        return quote! {
            pub fn #fn_name(options: dexterous_developer::HotReloadOptions) {
                #ast

                let initial = dexterous_developer::InitialPlugins::new(HotReloadPlugin);

                #fn_name(initial);
            }
        }
        .into();
    }
}
