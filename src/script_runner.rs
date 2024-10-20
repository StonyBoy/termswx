//Steen Hegelund
//Time-Stamp: 2024-Oct-17 11:13
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Run python scripts

use crate::term_switch::MsgType;
use crate::ansi_seq::AnsiSeqState;
use crate::config::subst_home;
use crate::console_service::show_error;

use log::{error, info, trace};
use std::io::{self, BufRead, BufReader, Write, Read};
use std::thread;
use std::thread::sleep;
use std::sync::Arc;
use std::process::{self, Child, ChildStdin};
use std::time::Duration;
use crossterm::terminal;
use crossterm::style::{Color, Stylize};
use crossbeam_channel::{Sender, Receiver, unbounded};
use std::sync::{atomic::AtomicU32, atomic::AtomicBool, atomic::Ordering};
use std::collections::HashMap;
use sysinfo::{Pid, Signal, System};

const SCRIPT_ALERT: char = '\u{11}';
const SCRIPT_MENU_TITLE: char = '\u{12}';
const SCRIPT_MENU_ITEM: char = '\u{13}';
const SCRIPT_MENU_PROMPT: char = '\u{14}';
const SCRIPT_USER_TEXT: char = '\u{15}';
const SCRIPT_BINARY_ON: char = '\u{16}';
const SCRIPT_BINARY_OFF: char = '\u{17}';

pub struct ScriptCommand {
    pub arg: String,
    pub tx: Sender<MsgType>,
    pub rx: Receiver<MsgType>,
    pub pid: Arc::<AtomicU32>,
    pub python: String,
    pub envir: HashMap<String, String>,
    pub in_prompt: Arc::<AtomicBool>,
    pub binary_mode: Arc::<AtomicBool>,
}


// Terminate a running/dead script using the process id
pub fn signal(u32pid: u32) {
    let pid = usize::try_from(u32pid).unwrap();
    info!("Kill script process id: {pid}");
    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid)) {
        #[cfg(target_os = "windows")]
        process.kill_with(Signal::Kill);
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        process.kill_with(Signal::Term);
    }
}


// Write a character to script stdin via a buffer
fn script_stdin_write(stdin: &mut ChildStdin, ch: u8, sleep: bool) -> bool {
    let mut buffer = vec![0; 1];

    buffer.clear();
    buffer.push(ch);
    loop {
        match stdin.write(&buffer) {
            Ok(0) => {
                stdin.flush().unwrap();
                if sleep {
                    thread::sleep(Duration::from_millis(6));
                }
                return true;
            }
            Ok(_) => {
                stdin.flush().unwrap();
                if sleep {
                    thread::sleep(Duration::from_micros(100));
                }
                return true;
            }
            Err(e) => {
                error!("\rScript stdin write error: {e:?}");
                break;
            }
        }
    }
    false
}


// Run a child process
fn child_process(cmd: ScriptCommand, mut child: Child) {
    cmd.pid.store(child.id(), Ordering::Relaxed);
    let mut stdin = child.stdin.take().expect("Get stdin");
    let mut stdout = child.stdout.take().expect("Get stdout");
    let stderr = child.stderr.take().expect("Get stderr");
    let (echo_tx, echo_rx) = unbounded();
    let mut fsm = AnsiSeqState::new();
    let binary_mode = cmd.binary_mode.clone();
    let endtext = format!("End {} with process id {}\r", cmd.arg, child.id());

    // Get serial output and send to the script stdin
    thread::spawn(move || {
        const CR: u8 = 0xd;
        loop {
            match cmd.rx.recv() {
                Ok(MsgType::Console(mut ch)) => {
                    if binary_mode.load(Ordering::Relaxed) {
                        trace!("Script_rx (binary): {:#02x} '{}'", ch, ch as char);
                        if !script_stdin_write(&mut stdin, ch, false) {
                            break;
                        }
                    } else {
                        trace!("Script_rx: {:#02x} '{}'", ch, ch as char);
                        // Remove command echo characters
                        if let Ok(val) = echo_rx.try_recv() {
                            if val == ch {
                                continue;
                            }
                        }
                        if ch == CR {
                            continue;
                        }
                        // Filter out ANSI escape sequence
                        while let Some(ch) = fsm.input(&mut ch) {
                            trace!("Script_rx_filtered: {:#02x} '{}'", ch, ch as char);
                            if !script_stdin_write(&mut stdin, ch, true) {
                                break;
                            }
                        }
                    }
                }
                Ok(MsgType::ScriptAlertResponse(ch)) => {
                    if !script_stdin_write(&mut stdin, ch, true) {
                        break;
                    }
                }
                Ok(MsgType::ScriptDone) => {
                    // The thread must exit to close the script_rx channel
                    // or there will be multiple receivers fighting for the same events
                    info!("Script Done received");
                    cmd.pid.store(0, Ordering::Relaxed);
                    break;
                }
                Ok(_) => (),
                Err(e) => {
                    error!("\rScript Error: {e:?}");
                    break;
                }
            }
        }
        info!("Script stdin thread done - wait for process termination");
        child.wait().expect("Wait for script to terminate");
        info!("Script process terminatated");
    });

    // Get script stdout and send it to the serial port
    let binary_mode = cmd.binary_mode.clone();
    thread::spawn(move || {
        const CR: u8 = 0xd;
        loop {
            let mut buffer = vec![0; 1024];
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    println!("\r{}\r", endtext);
                    info!("{}", endtext);
                    // Send done to force the stdout thread to exit too
                    cmd.tx.send(MsgType::ScriptDone).unwrap();
                    break;
                }
                Ok(cnt) => {
                    if binary_mode.load(Ordering::Relaxed) {
                        trace!("Script_tx (binary): count {}/{}", cnt, buffer.len());
                        for idx in 0..cnt {
                            let val: u8 = buffer[idx];
                            trace!("Script_tx (binary): {:#02x} '{}'", val, val as char);
                            cmd.tx.send(MsgType::Console(val)).unwrap();
                            // thread::sleep(Duration::from_micros(100));
                        }
                    } else {
                        for idx in 0..cnt {
                            let val: u8 = buffer[idx];
                            if val == CR {
                                continue;
                            }
                            trace!("Script_tx: {:#02x} '{}'", val, val as char);
                            cmd.tx.send(MsgType::Console(val)).unwrap();
                            echo_tx.send(val).unwrap();
                        }
                    }
                }
                Err(e) => {
                    error!("\rStdout Error: {e:?}");
                    break;
                }
            }
        }
        info!("script stdout thread done");
    });

    // Get script stderr and print it on the console as alert messages
    let in_prompt = cmd.in_prompt.clone();
    let binary_mode = cmd.binary_mode.clone();
    thread::spawn(move || {
        let mut rdr = BufReader::new(stderr);
        loop {
            let mut buf = String::new();
            match rdr.read_line(&mut buf) {
                Ok(0) => {
                    info!("Script end of stderr");
                    break;
                }
                Ok(_) => {
                    // First char is a prefix that identifies the type of message
                    let prompt = buf.chars().nth(0).unwrap();
                    // Remove the prefix and the terminating newline
                    let text = String::from(&buf[1..(buf.len()-1)]);
                    terminal::disable_raw_mode().unwrap();
                    match prompt {
                        SCRIPT_ALERT => {
                            println!("\n{}", text.with(Color::White).on(Color::DarkMagenta));
                        }
                        SCRIPT_MENU_TITLE => {
                            sleep(Duration::from_millis(200));
                            println!("\n\n{}", text.with(Color::White).on(Color::Green));
                        }
                        SCRIPT_MENU_ITEM => {
                            println!("{}", text.with(Color::Black).on(Color::DarkYellow));
                        }
                        SCRIPT_MENU_PROMPT => {
                            in_prompt.store(true, Ordering::Relaxed);
                            print!("{}", text.with(Color::Black).on(Color::DarkGreen));
                            io::stdout().flush().unwrap();
                        }
                        SCRIPT_USER_TEXT => {
                            println!("{}", text.with(Color::White).on(Color::DarkBlue));
                        }
                        SCRIPT_BINARY_ON => {
                            println!("\nbinary: on");
                            info!("binary on");
                            binary_mode.store(true, Ordering::Relaxed);
                        }
                        SCRIPT_BINARY_OFF => {
                            println!("\nbinary: off");
                            info!("binary off");
                            binary_mode.store(false, Ordering::Relaxed);
                        }
                        _ => {
                            let text = String::from(&buf[0..(buf.len()-1)]);
                            println!("{}", text.with(Color::White).on(Color::Black));
                        }
                    }
                    terminal::enable_raw_mode().unwrap();
                }
                Err(e) => {
                    error!("\rScript stderr error: {e:?}");
                    break;
                }
            }
        }
        info!("Script stderr thread done");
    });
}


// Execute a script and piping input/output via term server
pub fn execute_script(cmd: ScriptCommand) {
    // Replace "~" with the home folder for script paths
    let narg = subst_home(&cmd.arg);
    let args = narg.split(" ");
    let res = process::Command::new(cmd.python.clone())
        .arg("-u") // Unbuffered IO - Really important!
        .args(args)
        .envs(cmd.envir.clone())
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn();

    match res {
        Err(err) => {
            let msg = vec![
                format!("Could not start python script \"{}\"", cmd.arg),
                format!("Error: \"{}\"", err.to_string()),
            ];

            show_error(msg);
        }
        Ok(child) => {
            let text = format!("Start {} as process id {}", cmd.arg, child.id());
            println!("\r{}\r", text);
            info!("{}", text);
            child_process(cmd, child);
        }
    }
}
