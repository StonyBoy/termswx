//Steen Hegelund
//Time-Stamp: 2025-Apr-07 14:24
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :

// Configuration file stored as ~/.config/termswx/config.toml
//
// Command Line Arguments:
// - Connect console to a device for read/write: <devicepath|host:port>
// - Run TCP Server for local device: -p <port>
// - Run quiet TCP Server: -s
// - Start trace logging at loglevel: -v[v*] <filepath>
// - Specify tracefile name: -t <filepath>

use log::{info, trace};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs::canonicalize;

use std::path::PathBuf;
use std::time::Instant;
use std::env;
use bpaf::*;
use config::FileConfig;

mod logger_service;
mod console_service;
mod script_runner;
mod serial_service;
mod network_service;
mod term_switch;
mod ansi_filter;
mod config;

const CONFIG_VERSION: i64 = 7;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CmdLineConfig {
    device: PathBuf,
    baudrate: u32,
    portnum: u16,
    maxclients: i8,
    server: bool,
    tracefile: String,
    keeprunning: bool,
    enumerate: bool,
    verbose: usize,
    networkdev: bool,
    version: bool,
    start: Instant,
    config_version: i64,
    config_file: PathBuf,
}


fn parse_args() -> OptionParser<CmdLineConfig> {
    let device = positional::<PathBuf>("DEVICE/HOST")
        .help("Device path /dev/xxx or hostname:portnum")
        .complete_shell(bpaf::ShellComp::File { mask: None })
        .fallback("".into());

    let baudrate = short('b')
        .long("baudrate")
        .help("Set baudrate")
        .argument::<u32>("BAUDRATE")
        .fallback(115200)
        .display_fallback();

    let portnum = short('p')
        .long("portnum")
        .help("Run TCP Server listning on port")
        .argument::<u16>("PORTNUM")
        .guard(|&p| p > 1024, "PORTNUM must be greater than 1024")
        .fallback(0);

    let maxclients = short('m')
        .long("maxclients")
        .help("Maximum number of remote clients")
        .argument::<i8>("MAXCLIENTS")
        .fallback(1)
        .display_fallback();

    let keeprunning = short('k')
        .long("keeprunning")
        .help("Continue even if the port even disappears")
        .switch();

    let server = short('s')
        .long("server")
        .help("Activate quiet TCP Server mode (needs -p)")
        .switch();

    // Log Level error (highest priority), warn, info, debug, and trace
    let verbose = short('v')
        .long("verbose")
        .help("Increase the verbosity\n You can increase this up to 5 times")
        .req_flag(())
        .many()
        .map(|xs| xs.len())
        .guard(|&x| x <= 5, "This is the highest level");

    let tracefile = short('t')
        .long("trace")
        .help("Storing the termswx trace output")
        .argument::<String>("FILENAME")
        .fallback("/tmp/termswx_trace.log".to_string());

    let enumerate = short('e')
        .long("enumerate")
        .help("List available serial ports")
        .switch();

    let version = short('V')
        .long("version")
        .help("Show version information")
        .switch();

    let networkdev = pure(false);
    let start = pure(Instant::now());
    let config_version = pure(CONFIG_VERSION);

    let mut path = PathBuf::new();

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    path.push(env::var("XDG_CONFIG_HOME").expect("No XDG_CONFIG_HOME environment"));

    #[cfg(target_os = "windows")]
    path.push(env::var("APPDATA").expect("No APPDATA environment")); //  APPDATA=C:\Users\xxxx\AppData\Roaming

    path.push("termswx");
    let mut filename = path.clone();
    filename.push("config.toml");
    let config_file = pure(filename);

    construct!(CmdLineConfig {
        baudrate,
        portnum,
        maxclients,
        server,
        keeprunning,
        tracefile,
        verbose,
        enumerate,
        version,
        device,
        networkdev,
        start,
        config_version,
        config_file,
    })
    .to_options().descr("TermSWX - The Serial Terminal Switch")
}


#[cfg(any(target_os = "linux", target_os = "macos"))]
fn parse_path(opts: &mut CmdLineConfig) -> bool {
    let path = canonicalize(&opts.device);
    match path {
        Ok(_) => {
            true
        }
        Err(_) => {
            if !opts.device.starts_with("/") {
                opts.networkdev = true;
                return true;
            }
            false
        }
    }
}


#[cfg(target_os = "windows")]
fn parse_path(opts: &mut CmdLineConfig) -> bool {
    let comdev = String::from(opts.device.to_str().expect("Expected a COM port"));
    if !comdev.starts_with("COM") {
        opts.networkdev = true;
    }
    true
}


pub fn terminate(start: Instant, msg: &str) {
    trace!("Close console");
    console_service::close_console();
    let duration = start.elapsed();
    println!("TermSWX completed after {}s: {}", duration.as_secs(), msg);
    std::process::exit(1);
}


fn main() {
    let mut cmdopts = parse_args().run();

    if !parse_path(&mut cmdopts) {
        println!("Could not open device");
        return;
    }
    let fileconfig = FileConfig::new(&cmdopts.config_file, cmdopts.config_version, cmdopts.start);

    logger_service::init(cmdopts.tracefile.clone(), cmdopts.verbose);
    info!("{} {} started with options: {:?}",
          env!("CARGO_PKG_NAME"),
          env!("CARGO_PKG_VERSION"),
          cmdopts);

    if cmdopts.version {
        println!("{} version {} by {}",
                 env!("CARGO_PKG_NAME"),
                 env!("CARGO_PKG_VERSION"),
                 env!("CARGO_PKG_AUTHORS")
                 );
        return;
    }

    if cmdopts.enumerate {
        let ports = serialport::available_ports().expect("No ports found!");
        println!("Available Serial Ports:");
        for p in ports {
            println!(" - {}", p.port_name);
        }
        return;
    }
    if cmdopts.device.to_str().unwrap().len() == 0 {
        println!("Could not open device");
        return;
    }

    let mut termswx = term_switch::start(cmdopts.portnum > 0);
    let console = console_service::open_console(&mut termswx, &cmdopts, fileconfig);
    if cmdopts.networkdev {
        network_service::open_connection(&mut termswx, cmdopts.device, cmdopts.start);
    } else {
        if cmdopts.portnum > 0 {
            network_service::start_server(&mut termswx, cmdopts.portnum, cmdopts.maxclients, cmdopts.start);
        }
        serial_service::open_device(&mut termswx, cmdopts.device, cmdopts.baudrate, cmdopts.keeprunning, cmdopts.start);
    }
    trace!("Waiting for Console Thread");
    console.unwrap().join().unwrap();
    trace!("Stopping Terminal Service");
    console_service::close_console();
    termswx.stop();
    let duration = cmdopts.start.elapsed();
    println!("TermSWX completed after {}s", duration.as_secs());
}
