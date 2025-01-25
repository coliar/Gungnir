use core::future::Future;
use core::pin::Pin;
use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use crate::println;
use super::Executor;
use futures_channel::oneshot;


#[derive(Clone)]
pub(in crate::gsh) struct CmdEntry {
    summary: &'static str,
    future_fn: fn() -> Pin<Box<dyn Future<Output = ()>>>,
}

impl CmdEntry {
    pub(in crate::gsh) fn new(
        summary: &'static str, 
        future_fn: fn() -> Pin<Box<dyn Future<Output = ()>>>
    ) -> Self {
        CmdEntry { summary, future_fn }
    }
}

pub(super) struct GShell {
    cmds: BTreeMap<&'static str, CmdEntry>,
    executor: Option<Arc<Executor>>,
}

unsafe impl Sync for GShell {}
unsafe impl Send for GShell {}

impl GShell {
    pub(super) fn new() -> Self {
        GShell {
            cmds: BTreeMap::new(),
            executor: None,
        }
    }

    pub(super) fn set_exec(&mut self, executor: Arc<Executor>) {
        self.executor = Some(executor);
    }

    pub(super) fn add_cmd(&mut self, name: &'static str, cmd: CmdEntry) {
        match self.cmds.get(name) {
            Some(_) => {
                println!("Command {} already exists", name);
            }
            None => {
                self.cmds.insert(name, cmd);
            }
        }
    }

    pub(super) fn command(&self, cmd: &str, tx: oneshot::Sender<()>) {
        println!("\n");
        if cmd == "help" {
            for (name, entry) in self.cmds.iter() {
                println!("{}: {}", name, entry.summary);
            }
            tx.send(()).unwrap();
            return;
        }
        match self.cmds.get(cmd) {
            Some(entry) => {
                let future_fn = entry.future_fn;
                if let Some(executor) = &self.executor {
                    executor.spawn(async move {
                        future_fn().await;
                        tx.send(()).unwrap();
                    });
                } else {
                    println!("Executor not set");
                    tx.send(()).unwrap();
                }
            }
            None => {
                println!("Command {} not found", cmd);
                tx.send(()).unwrap();
            }
        }
    }

    pub(super) fn suggest(&self) -> String {
        todo!()
    }

    pub(super) fn bell(&self) {}
}