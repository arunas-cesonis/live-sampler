use nih_plug::nih_export_standalone;
use notify::{Config, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

use pyo3_plugin::PyO3Plugin;

fn main() {
    //    let (tx, rx) = crossbeam_channel::unbounded();
    //    let mut watcher = RecommendedWatcher::new(tx, Config::default().with_manual_polling()).unwrap();
    //    watcher
    //        .watch(Path::new("test.py"), RecursiveMode::NonRecursive)
    //        .unwrap();
    //
    //    rx.try_recv()
    //    for res in rx {
    //        match res {
    //            Ok(event) => {
    //                println!("{:?}", event);
    //            }
    //            Err(e) => {
    //                println!("watch error: {:?}", e);
    //            }
    //        }
    //    }
    nih_export_standalone::<PyO3Plugin>();
}
