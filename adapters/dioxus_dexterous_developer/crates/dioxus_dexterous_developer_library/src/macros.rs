pub use paste::paste;

#[cfg(not(feature = "hot"))]
mod cold {
    #[macro_export]
    macro_rules! reloadable_main {
        ($f:ident $body:block) => {
            fn reloadable_main_implementation() $body

            pub fn $f() {
                reloadable_main_implementation();
            }
        };
    }

    #[macro_export]
    macro_rules! reloadable_app {
        ($f: ident $body:block) => {
            struct $f;

            impl dioxus_dexterous_developer::ReloadableDioxusApp for $f {
                fn name() -> &'static str {
                    $f
                }

                fn call() -> dioxus::prelude::Element $body
            }
        };
    }
}

#[cfg(feature = "hot")]
mod hot {
    #[macro_export]
    macro_rules! reloadable_main {
        ($f:ident $body:block) => {
            fn reloadable_main_implementation() $body

            #[no_mangle]
            pub extern "Rust" fn dexterous_developer_instance_main(_: &mut ()) {
                println!("Setting Up With Hot Reload");
                reloadable_main_implementation();
            }

            pub fn $f() {
                println!("Setting Up Without Hot Reload");
                reloadable_main_implementation();
            }
        };
    }

    #[macro_export]
    macro_rules! reloadable_app {
        ($f: ident $body:block) => {
            dioxus_dexterous_developer::macros::paste!(reloadable_app!(@inner $f [<$f _dexterous_developer_inner>] $body););
        };

        (@inner $f: ident $internal:ident $body:block) => {
            struct $f;

            #[no_mangle]
            pub fn $internal() -> dioxus::prelude::Element $body

            impl dioxus_dexterous_developer::ReloadableDioxusApp for $f {
                fn name() -> &'static str {
                    stringify!($internal)
                }

                fn call() -> dioxus::prelude::Element {
                    $internal()
                }
            }
        };
    }
}
