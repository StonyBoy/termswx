//Steen Hegelund
//Time-Stamp: 2024-Nov-28 09:07
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Handle input from the local console and looking up keyboard shortcuts
// Also run python scripts

use crate::CmdLineConfig;
use crate::term_switch::{TermSwitch, MsgType};
use crate::config::{TermCommand, dump_keyseq, FileConfig, subst_home};
use crate::script_runner::{signal, ScriptCommand, execute_script};

use log::{error, trace};
use std::thread::sleep;
use std::io::{self, Write, Read};
use std::thread;
use std::time::Duration;
use std::env;
use crossterm::terminal;
use crossterm::execute;
use crossterm::style::{Color, Stylize};
use std::sync::{Arc, atomic::AtomicBool, atomic::AtomicI8, atomic::Ordering};
use std::collections::HashMap;


fn banner(cmdopts: &CmdLineConfig, helpkey: String) {
    let lines = vec![
        (true, 100, format!("=== Welcome to TermSWX").with(Color::White).on(Color::Black)),
        (cmdopts.portnum > 0, 100, format!(" => ").on(Color::White)),
        (cmdopts.portnum > 0, 100, format!("Listening on port {}", cmdopts.portnum).with(Color::DarkCyan).on(Color::White)),
        (cmdopts.networkdev, 100, format!(" => ").on(Color::White)),
        (cmdopts.networkdev, 100, format!("Connected to {:?}", cmdopts.device).with(Color::Red).on(Color::White)),
        (true, 100, format!(" => ").on(Color::White)),
        (true, 100, format!("Use {} to get help ===\r", helpkey).with(Color::White).on(Color::Black)),
    ];
    for (enabled, ms, line) in lines.iter() {
        if *enabled {
            print!("{}", line);
            std::io::stdout().flush().unwrap();
            sleep(Duration::from_millis(*ms));
        }
    }
    println!("");
}


// Do not process input when in silent server mode
fn wait_for_exit(fileconfig: FileConfig) -> Result<thread::JoinHandle<()>,u32> {
    let thr = thread::spawn(move || {
        let mut buffer = vec![0; 80];
        loop {
            let cnt = io::stdin().read(&mut buffer).unwrap();
            if let Some(cmd) = fileconfig.find_shortcut(&buffer, cnt) {
                match cmd {
                    TermCommand::Quit => {
                        trace!("Silent Server Quit");
                        break;
                    }
                    _ => (),
                }
            }
        }
    });
    Ok(thr)
}


pub fn show_error(msg: Vec<String>) {
    terminal::disable_raw_mode().unwrap();
    println!("");
    for line in msg {
        println!("{}", line.with(Color::White).on(Color::DarkRed));
    }
    println!("");
    terminal::enable_raw_mode().unwrap();
}


// Use the alternate screen for output
fn show_help(cmdopts: &CmdLineConfig, fileconfig: &FileConfig, clients: &Arc<AtomicI8>) {
    terminal::disable_raw_mode().unwrap();
    execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen).unwrap();
    let size = crossterm::terminal::size().unwrap();

    println!("{}", format!("\n").on(Color::White));
    println!("{}", format!("=== TermSWX Help Menu").with(Color::White).on(Color::DarkGreen));
    println!("{}", format!("  {}: v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));
    if cmdopts.portnum > 0 {
        println!("{}", format!("  Server portnumber: {}", cmdopts.portnum));
    }
    println!("{}", format!("  Connected to: {:?}", cmdopts.device));
    println!("{}", format!("  Remote clients: {} of maximum {}", clients.load(Ordering::Relaxed), cmdopts.maxclients));
    let configfile = cmdopts.config_file.clone().into_os_string().into_string().unwrap();
    println!("{}", format!("  Tracefile: {}", cmdopts.tracefile));
    println!("{}", format!("  Configurationfile: {}", configfile));
    println!("{}", format!("  Terminal size: {:?}", size));
    let duration = cmdopts.start.elapsed();
    println!("{}", format!("  Elapsed Time: {}s", duration.as_secs()));

    println!("{}", format!("\n").on(Color::White));
    println!("{}", format!("=== Keyboard Shortcuts").with(Color::White).on(Color::DarkGreen));

    fileconfig.print_shortcuts();
    println!("  ESC => close this help");
    terminal::enable_raw_mode().unwrap();
    loop {
        let mut buffer = vec![0; 80];
        let cnt = io::stdin().read(&mut buffer).unwrap();
        if let Some(cmd) = fileconfig.find_shortcut(&buffer, cnt) {
            match cmd {
                TermCommand::HelpMenu => break,
                _ => (),
            }
        } else {
            if cnt == 1 && buffer[0] == 0x1b {
                break;
            } else {
                println!("\rKeyseq: {}", dump_keyseq(&buffer[0..cnt]));
            }
        }
    }
    execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen).unwrap();
}


fn build_script_envir(cmdopts: &CmdLineConfig, fileconfig: &FileConfig) -> HashMap<String, String> {
    // Get Terminal Size into an environment variable
    let size = format!("{:?}", crossterm::terminal::size().unwrap());
    // Get Terminal device into an environment variable
    let device = String::from(cmdopts.device.to_str().unwrap());
    // Get Terminal device baudrate into an environment variable
    let baudrate = format!("{}", cmdopts.baudrate);

    let mut envir = HashMap::new();
    envir.insert("TERMSWX_SIZE".to_string(), size);
    envir.insert("TERMSWX_DEV".to_string(), device);
    envir.insert("TERMSWX_BAUDRATE".to_string(), baudrate);
    if let Some(mut eiter) = fileconfig.get_enviroment() {
        for (key, value) in eiter.by_ref() {
            envir.insert(key.to_string(), value.to_string());
        }
    }
    envir
}


// Open the local console for reading input.  Handle shortcuts: injecting or starting a script
// Use the raw mode (no wait for enter, no automatic output)
pub fn open_console(termswx: &mut TermSwitch, cmdopts: &CmdLineConfig, fileconfig: FileConfig) -> Result<thread::JoinHandle<()>,u32> {
    const NL: u8 = 0xa;
    const CR: u8 = 0xd;
    trace!("Starting console thread: quiet: {}", cmdopts.server);
    terminal::enable_raw_mode().unwrap();

    banner(&cmdopts, fileconfig.find_command(TermCommand::HelpMenu).expect("Found no helpkey in the configuration"));
    // Running a silent server -> no keyboard handling except exit
    if cmdopts.server {
        return wait_for_exit(fileconfig);
    }

    let switch_tx = termswx.get_switch_tx();
    let console_rx = termswx.get_console_rx();
    let script_rx = termswx.get_script_rx();
    let script_pid = termswx.get_script_pid();
    let binary_mode = termswx.get_binary_mode();
    let clients = termswx.get_clients();

    // Process keyboard input
    let thropts = cmdopts.clone();
    let thr = thread::spawn(move || {
        let mut buffer = vec![0; 80];
        let in_prompt: Arc::<AtomicBool> = Arc::new(AtomicBool::new(false));
        loop {
            let cnt = io::stdin().read(&mut buffer).unwrap();
            trace!(" - chars {}", dump_keyseq(&buffer[0..cnt]));
            if let Some(cmd) = fileconfig.find_shortcut(&buffer, cnt) {
                match cmd {
                    TermCommand::HelpMenu => show_help(&thropts, &fileconfig, &clients),
                    TermCommand::Nop => (),
                    TermCommand::Quit => {
                        trace!("Console Quit");
                        break;
                    }
                    TermCommand::StopScript => {
                        trace!("Kill Script with pid: {}", script_pid.load(Ordering::Relaxed));
                        if script_pid.load(Ordering::Relaxed) != 0 {
                            signal(script_pid.load(Ordering::Relaxed));
                        }
                    }
                    TermCommand::SerialBreak => {
                        trace!("Send SerialBreak");
                        switch_tx.send(MsgType::SerialBreak).unwrap();
                    }
                    TermCommand::Inject(seq) => {
                        for val in seq {
                            switch_tx.send(MsgType::Console(*val)).unwrap();
                        }
                    }
                    TermCommand::Prompt(arg) => {
                        println!("{}\r", format!("{}", arg).with(Color::White).on(Color::DarkGreen));
                    }
                    TermCommand::FileInject(arg) => {
                        let pid = script_pid.load(Ordering::Relaxed);
                        if pid != 0 {
                            println!("{}", format!("Error: Script with PID {:?} already running", pid).with(Color::Red));
                        } else {
                            // Replace "~" with the home folder for script paths
                            let narg = subst_home(arg);

                            if let Ok(content) = std::fs::read_to_string(narg.clone()) {
                                for val in content.as_bytes() {
                                    switch_tx.send(MsgType::Console(*val)).unwrap();
                                    if *val == NL {
                                        thread::sleep(Duration::from_millis(250));
                                    }
                                }
                            } else {
                                println!("Could not run script: {}", narg);
                            }

                        }
                    }
                    TermCommand::RunScript(arg) => {
                        let pid = script_pid.load(Ordering::Relaxed);
                        if pid != 0 {
                            println!("{}", format!("Error: Script with PID {:?} already running", pid).with(Color::Red));
                        } else {
                            let cmd = ScriptCommand {
                                tx: switch_tx.clone(),
                                rx: script_rx.clone(),
                                pid: script_pid.clone(),
                                arg: arg.to_string().clone(),
                                python: fileconfig.get_python(),
                                envir: build_script_envir(&thropts, &fileconfig),
                                in_prompt: in_prompt.clone(),
                                binary_mode: binary_mode.clone(),
                            };
                            execute_script(cmd);
                        }
                    }
                    TermCommand::SttySize => {
                        let size = terminal::size().unwrap();
                        let cmd = format!("stty cols {} rows {}\r", size.0 - 1, size.1 - 1);
                        for val in cmd.as_bytes() {
                            switch_tx.send(MsgType::Console(*val)).unwrap();
                        }
                    }
                    TermCommand::Environment => {
                        if let Some(mut eiter) = fileconfig.get_enviroment() {
                            for (key, value) in eiter.by_ref() {
                                let cmd = format!("export {key}={value}\r");
                                for val in cmd.as_bytes() {
                                    switch_tx.send(MsgType::Console(*val)).unwrap();
                                }

                            }
                        }
                    }
                }
            } else {
                if in_prompt.load(Ordering::Relaxed) {
                    for idx in 0..cnt {
                        let val: u8 = buffer[idx];

                        switch_tx.send(MsgType::ScriptAlertResponse(val)).unwrap();
                        let out: &[u8] = &buffer[idx..idx+1];
                        io::stdout().write(out).unwrap();
                        io::stdout().flush().unwrap();
                        if val == CR {
                            in_prompt.store(false, Ordering::Relaxed);
                        }
                    }
                } else {
                    for idx in 0..cnt {
                        let val: u8 = buffer[idx];
                        match switch_tx.send(MsgType::Console(val)) {
                            Ok(_) => (),
                            Err(_) => {
                                error!("Cannot send console input to term_switch");
                            }
                        }
                    }
                }
            }
        }
    });

    // Send responses to stdout (eg echo from the serial port)
    thread::spawn(move || {
        let mut buffer = vec![0; 2];
        loop {
            match console_rx.recv() {
                Ok(MsgType::Console(ch)) => {
                    #[cfg(target_os = "windows")]
                    if ch >= 0x80 {
                        continue;
                    }
                    buffer.clear();
                    buffer.push(ch);
                    match io::stdout().write(&buffer) {
                        Ok(_) => (),
                        Err(e) => {
                            error!("Receive Console Error: {e:?}");
                        }
                    }
                }
                Ok(MsgType::Exit) => {
                    trace!("Console Exit received");
                    break;
                }
                Ok(_) => (),
                Err(e) => {
                    error!("Receive Error: {e:?}");
                }
            }
            match io::stdout().flush() {
                Ok(_) => (),
                Err(e) => {
                    error!("Flush Error: {e:?}");
                }
            }
        }
    });
    Ok(thr)
}


// Disable raw input mode
pub fn close_console() {
    terminal::disable_raw_mode().unwrap();
}
