use core::future::Future;
use core::pin::Pin;
use alloc::{boxed::Box, collections::{btree_map::BTreeMap, vec_deque::VecDeque}, string::{String, ToString}, sync::Arc};
use crate::println;
use super::Executor;
use futures_channel::oneshot;


#[derive(Clone)]
pub(in crate::gsh) struct CmdEntry {
    summary: &'static str,
    future_fn: fn(VecDeque<String>) -> Pin<Box<dyn Future<Output = ()>>>,
}

impl CmdEntry {
    pub(in crate::gsh) fn new(
        summary: &'static str, 
        future_fn: fn(VecDeque<String>) -> Pin<Box<dyn Future<Output = ()>>>
    ) -> Self {
        CmdEntry { summary, future_fn }
    }
}

pub(super) struct GShell {
    cmds: BTreeMap<String, CmdEntry>,
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
                self.cmds.insert(name.to_string(), cmd);
            }
        }
    }

    pub(super) fn command(&self, line: &str, tx: oneshot::Sender<()>) {
        println!("\n");
        let mut words = split_to_words(line);
        let cmd = words.pop_front().expect("empty shell line");
        let params = words;
        if cmd == "help" {
            for (name, entry) in self.cmds.iter() {
                println!("{}: {}", name, entry.summary);
            }
            tx.send(()).unwrap();
            return;
        }
        match self.cmds.get(&cmd) {
            Some(entry) => {
                let future_fn = entry.future_fn;
                if let Some(executor) = &self.executor {
                    executor.spawn(async move {
                        future_fn(params).await;
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

fn split_to_words(line: &str) -> VecDeque<String> {
    line.split_ascii_whitespace()
        .map(|s| s.to_string())
        .collect()
}