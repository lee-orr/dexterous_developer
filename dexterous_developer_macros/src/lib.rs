extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
#[allow(clippy::needless_return)]
pub fn bevy_app_setup(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = parse_macro_input!(item as ItemFn);

    let fn_name: &proc_macro2::Ident = &ast.sig.ident;
    let input = &ast.sig.inputs.first().expect("Should have an input");
    let input = match input {
        syn::FnArg::Receiver(_) => panic!("bevy main shouldn't have a self input"),
        syn::FnArg::Typed(v) => &v.pat,
    };
    let block = &ast.block;

    let mut stream: Vec<TokenStream> = vec![];
    #[cfg(feature = "hot_internal")]
    {
        stream.push(quote!{
                #[no_mangle]
                pub extern "system" fn dexterous_developer_internal_main_inner_function(#input:  dexterous_developer::bevy_support::HotReloadableAppInitializer) 
                #block

                #[no_mangle]
                pub extern "system" fn dexterous_developer_internal_main(library_paths: std::ffi::CString, closure: fn() -> ()) {

                    dexterous_developer::bevy_support::build_reloadable_frame(library_paths, closure, |input| dexterous_developer_internal_main_inner_function(input));
                }
            }.into());
    }
    stream.push(
        quote! {
            pub fn #fn_name() {
                fn dexterous_developer_internal_main_inner_function<'a>(#input: dexterous_developer::InitialPluginsEmpty<'a>)
                #block

                let mut app = App::new();

                dexterous_developer_internal_main_inner_function(dexterous_developer::InitialPluginsEmpty::new(&mut app));

                app.run();
            }
        }
        .into(),
    );

    TokenStream::from_iter(stream)
}
