use alloc::{string::String, sync::Arc, vec::Vec};
use futures_util::StreamExt;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{driver::usart::UsartCodeStream, print, task::executor::Executor};
use gshell::CmdEntry;
use futures_channel::oneshot;

mod cmds;
mod gshell;

const UP: u8 = 0x41;
const DOWN: u8 = 0x42;
const RIGHT: u8 = 0x43;
const LEFT: u8 = 0x44;
const BS: u8 = 0x8; // Backspace
const DEL: u8 = 0x7f;
const TAB: u8 = 0x9;
const ESC: u8 = 0x1b;

lazy_static! {
    static ref GSHELL: Mutex<gshell::GShell> = Mutex::new(gshell::GShell::new());
}

fn register_cmd(name: &'static str, cmd: CmdEntry) {
    GSHELL.lock().add_cmd(name, cmd);
}

pub(crate) async fn gshell(executor: Arc<Executor>) {
    GSHELL.lock().set_exec(executor);
    cmds::add_cmds();

    let mut line = String::with_capacity(64);
    let mut cursor: usize = 0;
    let mut usart_code_stream = UsartCodeStream::new();
    let mut esc: bool = false;
    let mut ctl: bool = false;
    let mut history_vec: Vec<String> = Vec::new();
    let mut history_cursor: usize = 0;

    print!("#> ");

    while let Some(code) = usart_code_stream.next().await {
        if code == ESC {
            esc = true;
        } else if code == b'[' && esc {
            ctl = true;
        } else if ctl {
            esc = false;
            ctl = false;
            if code == UP {
                if history_cursor > 0 {
                    history_cursor -= 1;
                    line.clear();
                    line.push_str(&history_vec[history_cursor]);
                    cursor = line.len();
                    print!("\x1b[2K\x1b[0G");
                    print!("#> {}", line);
                }
            } else if code == DOWN {
                if history_cursor < history_vec.len() - 1 {
                    history_cursor += 1;
                    line.clear();
                    line.push_str(&history_vec[history_cursor]);
                    cursor = line.len();
                    print!("\x1b[2K\x1b[0G");
                    print!("#> {}", line);
                }
            } else if code == LEFT {
                if cursor > 0 {
                    cursor -= 1;
                    print!("\x1b[D");
                }
            } else if code == RIGHT {
                if cursor < line.len() {
                    cursor += 1;
                    print!("\x1b[C");
                }
            } else {
                print!("Gshell got unknown ctl code: {}\n", code);
            }
        } else if esc {
            esc = false;
            ctl = false;
        } else if code == TAB {
            GSHELL.lock().suggest();
        } else if code == DEL || code == BS {
            if cursor == 0 {
                GSHELL.lock().bell();
                continue;
            } else if line.len() == cursor {
                line.pop();
                cursor -= 1;
                print!("\u{08} \u{08}");
            } else {
                line.remove(cursor - 1);
                cursor -= 1;
                print!("\x1b[D\x1b[s\x1b[K");
                print!("{}", &line[cursor..]);
                print!("\x1b[u");
            }
        } else if code == b'\r' {
            if !line.is_empty() {
                let (tx, rx) = oneshot::channel();
                GSHELL.lock().command(&line, tx);
                rx.await.unwrap();
                if &line != history_vec.last().unwrap_or(&String::new()) {
                    history_vec.push(line.clone());
                }
                history_cursor = history_vec.len();
                line.clear();
                cursor = 0;
            }
            print!("\n#> ");
        } else {
            if code.is_ascii_control() {
                print!("\nGshell got unsupported symbol: {}\n", code);
            } else {
                if cursor == line.len() {
                    line.push(code as char);
                    print!("{}", code as char);
                    cursor += 1;
                } else {
                    line.insert(cursor, code as char);
                    cursor += 1;
                    print!("{}", code as char);
                    print!("\x1b[s\x1b[K");
                    print!("{}", &line[cursor..]);
                    print!("\x1b[u");
                }
            }
        }
    }
}
