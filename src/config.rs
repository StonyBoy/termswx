//Steen Hegelund
//Time-Stamp: 2024-Nov-28 18:54
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Maintain configuration file and parse keyboard shortcuts

use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use std::iter::zip;
use std::num::Wrapping;
use log::trace;
use std::env;


// Supported commands for keyboard shortcuts
#[derive(Debug, PartialEq, Eq)]
pub enum TermCommand {
    HelpMenu,
    Nop,
    Quit,
    StopScript,
    SerialBreak,
    Inject(Vec<u8>),
    FileInject(String),
    RunScript(String),
    SttySize,
    Environment,
    Prompt(String),
}


// Sequence of keys and matching command
pub struct ShortCut {
    pub keyname: String,
    pub keyseq: Vec<u8>,
    pub command: TermCommand,
}

// Provide a new KeyConfig type
pub type KeyConfig = Vec<ShortCut>;


pub struct FileConfig {
    pub shortcuts: KeyConfig,
    pub config: toml::Table,
}

impl FileConfig {
    // Load the config file and create the KeyConfig and return it
    pub fn new(config_file: &PathBuf, config_version: i64, start: Instant) -> FileConfig {
        let contents = fs::read_to_string(config_file.clone());

        match contents {
            Ok(content) => {
                let config = content.parse::<toml::Table>().expect("Error in the configuration file");
                Self::validate_version(config_file, config_version, start, &config);
                FileConfig {
                    shortcuts: create_keymap(&config),
                    config,
                }
            }
            Err(_) => {
                let mut path = config_file.clone();
                path.pop();
                fs::create_dir_all(path).unwrap();
                let config = create_new_config(&config_file, config_version);
                println!("Creating config file {:?}", config_file);
                FileConfig {
                    shortcuts: create_keymap(&config),
                    config,
                }
            }
        }
    }

    fn validate_version(config_file: &PathBuf, config_version: i64, start: Instant, config: &toml::Table) {
        if let Some(toml::Value::Table(general)) = config.get("general") {
            if let Some(toml::Value::Integer(version)) = general.get("version") {
                if *version != config_version {
                    crate::terminate(start, format!("Incorrect configuration file version: {}, expected: {} in {:?}",
                                                    version,
                                                    config_version,
                                                    config_file).as_str());
                }
            } else {
                crate::terminate(start, format!("Missing configuration file version: {} in {:?}",
                                                config_version,
                                                config_file).as_str());
            }
        } else {
            crate::terminate(start, format!("Missing configuration file version: {} in {:?}",
                                            config_version,
                                            config_file).as_str());
        }

    }

    // Lookup the keysequence and return the index of the shortcut in the KeyConfig
    pub fn find_shortcut(&self, buffer: &Vec<u8>, cnt: usize) -> Option<&TermCommand> {
        for elem in self.shortcuts.iter() {
            // Only compare keysequences with the same number of keys
            if elem.keyseq.len() == cnt {
                // Compare keysequences and return true if they match
                let found = zip(&elem.keyseq, buffer).map(|elem| u8diff(elem)).filter(|val| *val != 0).collect::<Vec<u8>>().len() == 0;
                trace!(" * compare to keyseq {:?} Command: {:?}: found: {}\r", elem.keyseq, elem.command, found);
                if found {
                    return Some(&elem.command)
                }
            }
        }
        None
    }

    pub fn find_command(&self, cmd: TermCommand) -> Option<String> {
        for elem in self.shortcuts.iter() {
            if elem.command == cmd {
                return Some(elem.keyname.clone());
            }
        }
        None
    }

    pub fn print_shortcuts(&self) {
        for elem in self.shortcuts.iter() {
            println!("  {} => {:?}", elem.keyname, to_command(elem));
        }
    }

    pub fn get_enviroment(&self) -> Option<toml::map::Iter<'_>> {
        if let Some(toml::Value::Table(envir)) = self.config.get("environment") {
            return Some(envir.iter());
        }
        None
    }

    pub fn get_python(&self) -> String {
        if let Some(toml::Value::Table(scripting)) = self.config.get("scripting") {
            if let Some(toml::Value::String(pyexe)) = scripting.get("python") {
                return pyexe.to_string();
            }
        }
        String::from("/usr/bin/python3")
    }
}

// Write default config in toml file an return it for immediate use
fn create_new_config(filename: &PathBuf, config_version: i64) -> toml::Table {

    let mut config = toml::toml! {
        [general]
            "version" = config_version
        [environment]
            "TERM" = "xterm"
        [scripting]
            "python" = "python3"
        [keynames]
            "F1" = "\x1bOP"
            "F2" = "\x1bOQ"
            "F3" = "\x1bOR"
            "F4" = "\x1bOS"
            "F5" = "\x1b[15~"
            "F6" = "\x1b[17~"
            "F7" = "\x1b[18~"
            "F8" = "\x1b[19~"
            "F9" = "\x1b[20~"
            "F10" = "\x1b[21~"
            "F11" = "\x1b[23~"
            "F12" = "\x1b[24~"
            "Print" = "\x1b[57361u"
            "Scroll" = "\x1b[57359u"
            "Pause" = "\x1b[57362u"
            "Del" = "\x7f"
        [keymap]
            "Ctrl+q" = "quit"
            "Ctrl+x" = "stop"
            "Ctrl+b" = "break"
            "Del" = "inject \x08" // Map DEL to Backspace
            "Ctrl+w" = "help"
            "Ctrl+t" = "sttysize"
            "Ctrl+e" = "environment"
            "Ctrl+o" = "inject cat /proc/meminfo\n"
            "Ctrl+p" = "run test.py --count 2 username password"
            "Ctrl+f" = "file test.sh"
            "Ctrl+r" = "prompt ---------- New Session ----------"
            "Print" = "nop"
            "Scroll" = "nop"
            "Pause" = "break"
    };

    // Add ctrl key sequences from a to z
    if let Some(&mut toml::Value::Table(ref mut keynames)) = config.get_mut("keynames") {
        for key in 'a'..='z' {
            let idx = (key as u8) - ('a' as u8) + 1;
            let ch: char = std::char::from_u32(idx.into()).unwrap();
            keynames.insert(format!("Ctrl+{key}"), toml::Value::String(ch.to_string()));
        }
    }

    let toml = toml::to_string(&config).unwrap();
    fs::write(filename, toml).unwrap();
    config
}


pub fn to_keyseq<'a>(config: &'a toml::Table, key: &String) -> Option<&'a [u8]> {
    if let Some(knames) = config.get("keynames") {
        if let Some(keynames) = knames.as_table() {
            for (name, value) in keynames.iter() {
                if key == name {
                    if let Some(seq) = value.as_str() {
                        return Some(seq.as_bytes());
                    }
                }
            }
        }
    }
    None
}


// Create a KeyConfig from a toml config
fn create_keymap(config: &toml::Table) -> KeyConfig {
    let mut keyconfig = Vec::new();
    if let Some(kopt) = config.get("keymap") {
        if let Some(keymap) = kopt.as_table() {
            for (key, value) in keymap.iter() {
                if let Some(keyseq) = to_keyseq(config, key) {
                    if let Some(cmdstr) = value.as_str() {
                        // One command word
                        match cmdstr {
                            "help" => {
                                keyconfig.push(ShortCut {
                                    keyname: key.to_string(),
                                    keyseq: keyseq.into(),
                                    command: TermCommand::HelpMenu,
                                });
                            }
                            "nop" => {
                                keyconfig.push(ShortCut {
                                    keyname: key.to_string(),
                                    keyseq: keyseq.into(),
                                    command: TermCommand::Nop,
                                });
                            }
                            "stop" => {
                                keyconfig.push(ShortCut {
                                    keyname: key.to_string(),
                                    keyseq: keyseq.into(),
                                    command: TermCommand::StopScript,
                                });
                            }
                            "break" => {
                                keyconfig.push(ShortCut {
                                    keyname: key.to_string(),
                                    keyseq: keyseq.into(),
                                    command: TermCommand::SerialBreak,
                                });
                            }
                            "sttysize" => {
                                keyconfig.push(ShortCut {
                                    keyname: key.to_string(),
                                    keyseq: keyseq.into(),
                                    command: TermCommand::SttySize,
                                });
                            }
                            "environment" => {
                                keyconfig.push(ShortCut {
                                    keyname: key.to_string(),
                                    keyseq: keyseq.into(),
                                    command: TermCommand::Environment,
                                });
                            }
                            "quit" => {
                                keyconfig.push(ShortCut {
                                    keyname: key.to_string(),
                                    keyseq: keyseq.into(),
                                    command: TermCommand::Quit,
                                });
                            }
                            _ => (),
                        }
                        if let Some((cmd, arg)) = cmdstr.split_once(' ') {
                            // Command word and arguments
                            match cmd {
                                "prompt" => {
                                    let text = String::from(arg);
                                    keyconfig.push(ShortCut {
                                        keyname: key.to_string(),
                                        keyseq: keyseq.into(),
                                        command: TermCommand::Prompt(text),
                                    });
                                }
                                "inject" => {
                                    let args: Vec<u8> = arg.bytes().collect();
                                    keyconfig.push(ShortCut {
                                        keyname: key.to_string(),
                                        keyseq: keyseq.into(),
                                        command: TermCommand::Inject(args),
                                    });
                                }
                                "file" => {
                                    let filename = String::from(arg);
                                    keyconfig.push(ShortCut {
                                        keyname: key.to_string(),
                                        keyseq: keyseq.into(),
                                        command: TermCommand::FileInject(filename),
                                    });
                                }
                                "run" => {
                                    let filename = String::from(arg);
                                    keyconfig.push(ShortCut {
                                        keyname: key.to_string(),
                                        keyseq: keyseq.into(),
                                        command: TermCommand::RunScript(filename),
                                    });
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
    }
    keyconfig
}


// Return abs(x - y) when x and y are u8 values
fn u8diff(elem: (&u8, &u8)) -> u8 {
    (Wrapping(*elem.0) - Wrapping(*elem.1)).0
}


//
// Convert a byte slice to a string containing both the hex values and the ascii characters (mixed in)
pub fn dump_keyseq(seq: &[u8]) -> String {
    let mut res: Vec<String> = Vec::new();
    res.push("[".to_string());
    for (idx, val) in seq.iter().enumerate() {
        let ch = *val as char;
        if ch.is_alphanumeric() || ch.is_ascii_punctuation() {
            res.push(format!("'{}'", ch));
        } else {
            res.push(format!("{:#02x}", val));
        }
        if idx != seq.len() - 1 {
            res.push(", ".to_string());
        }
    }
    res.push("]".to_string());
    res.join("")
}


pub fn to_command(shortcut: &ShortCut) -> String {
    match &shortcut.command {
        TermCommand::Inject(arg) => {
            match std::str::from_utf8(&arg) {
                Ok(cmd) => format!("Inject '{}'", cmd),
                Err(_) => format!("Invalid string"),
            }
        }
        _ => format!("{:?}", shortcut.command),
    }
}


// Replace "~" with the home folder in a string
pub fn subst_home(arg: &String) -> String {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let home = env::var("HOME").expect("Missing 'HOME' environment variable");

    #[cfg(target_os = "windows")]
    let home = env::var("USERPROFILE").expect("Missing 'USERPROFILE' environment variable");

    arg.replace("~", &home)
}
