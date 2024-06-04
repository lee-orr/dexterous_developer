use std::env::current_exe;

use camino::Utf8PathBuf;
use dexterous_developer_types::Target;

use crate::{
    dylib_runner_message::{DylibRunnerMessage, DylibRunnerOutput},
    run_app,
};

pub fn run_example_for_test(
    example: &'static str,
    messaging: fn(
        Box<dyn FnMut(&str)>,
        async_channel::Sender<DylibRunnerMessage>,
        async_channel::Receiver<DylibRunnerOutput>,
    ),
) {
    let path = current_exe().unwrap();
    let library_path = path.parent().unwrap().parent().unwrap().join("examples");

    let target = Target::current().unwrap();

    run_app(move |tx, rx| {
        let library_path = library_path.clone();
        Ok(std::thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut build_id = 0;
                    let example = target.dynamic_lib_name(example);
                    let lib = Utf8PathBuf::from_path_buf(library_path.join(example)).unwrap();
                    let _ = tx.send(DylibRunnerMessage::LoadRootLib {
                        build_id,
                        local_path: lib,
                    });
                    {
                        let tx = tx.clone();
                        let tx1 = tx.clone();
                        messaging(
                            Box::new(move |reload| {
                                let reload = target.dynamic_lib_name(reload);
                                let lib =
                                    Utf8PathBuf::from_path_buf(library_path.join(reload)).unwrap();
                                let _ = tx.send(DylibRunnerMessage::LoadRootLib {
                                    build_id: build_id + 1,
                                    local_path: lib,
                                });
                                build_id = build_id + 1;
                            }),
                            tx1,
                            rx,
                        );
                    }
                    let _ = tx.send(DylibRunnerMessage::ConnectionClosed).await;
                    Ok(())
                })
        }))
    })
    .unwrap();
}
