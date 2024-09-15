//Steen Hegelund
//Time-Stamp: 2024-May-02 22:32
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :

use log::{error, trace, info};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};
use std::sync::{Arc, atomic::AtomicU32, atomic::Ordering};
use std::net::SocketAddr;

// Messages sent via channels between threads
#[derive(Debug)]
pub enum MsgType {
    Add(SocketAddr),
    Added(Receiver<MsgType>),
    Remove(SocketAddr),
    Console(u8),
    Serial(u8),
    SerialClose,
    SerialBreak,
    ScriptAlertResponse(u8),
    ScriptDone,
    Exit,
}


#[derive(Debug)]
struct NetClient {
    addr: SocketAddr,
    tx: Sender<MsgType>,
}


// State for the TermSwitch service
pub struct TermSwitch {
    switch_tx: Sender<MsgType>,
    console_rx: Receiver<MsgType>,
    serial_rx: Receiver<MsgType>,
    network_rx: Receiver<MsgType>,
    script_rx: Receiver<MsgType>,

    script_pid: Arc<AtomicU32>,

    stop: bool,
}


// Public interface to get the receiver channels and stop the service
impl TermSwitch {
    pub fn stop(&mut self) {
        self.stop = true;
    }
    pub fn get_switch_tx(&self) -> Sender<MsgType> {
        self.switch_tx.clone()
    }
    pub fn get_serial_rx(&self) -> Receiver<MsgType> {
        self.serial_rx.clone()
    }
    pub fn get_console_rx(&self) -> Receiver<MsgType> {
        self.console_rx.clone()
    }
    pub fn get_network_rx(&self) -> Receiver<MsgType> {
        self.network_rx.clone()
    }
    pub fn get_script_rx(&self) -> Receiver<MsgType> {
        self.script_rx.clone()
    }
    pub fn get_script_pid(&self) -> Arc::<AtomicU32> {
        self.script_pid.clone()
    }
}


// Start TermSwitch Service
pub fn start(server: bool) -> TermSwitch {
    trace!("Starting Terminal Service");
    let (switch_tx, switch_rx) = unbounded();
    let (console_tx, console_rx) = unbounded();
    let (serial_tx, serial_rx) = unbounded();
    let (network_tx, network_rx) = unbounded();
    let (script_tx, script_rx) = unbounded();

    let termswx = TermSwitch {
        switch_tx,
        console_rx,
        serial_rx,
        network_rx,
        script_rx,
        script_pid: Arc::new(AtomicU32::new(0)),
        stop: false,
    };

    let script_pid = termswx.script_pid.clone();

    // Exchange messages
    thread::spawn(move || {
        let mut net_clients: Vec<NetClient> = Vec::new();

        loop {
            if termswx.stop {
                return;
            }
            // We need to decide the origin
            match switch_rx.recv() {
                Ok(MsgType::Add(addr)) => {
                    let (tx, rx) = unbounded();
                    net_clients.push(NetClient {
                        addr: addr.clone(),
                        tx,
                    });
                    network_tx.send(MsgType::Added(rx)).unwrap();
                }
                Ok(MsgType::Added(_)) => (),
                Ok(MsgType::Remove(addr)) => {
                    for (idx, elem)  in net_clients.iter().enumerate() {
                        if elem.addr == addr {
                            net_clients.remove(idx);
                            break;
                        }
                    }
                }
                Ok(MsgType::Console(ch)) => {
                    trace!("console: {ch:#02x}");
                    serial_tx.send(MsgType::Serial(ch)).unwrap();
                }
                Ok(MsgType::SerialClose) => {
                    trace!("serial close");
                    serial_tx.send(MsgType::SerialClose).unwrap();
                }
                Ok(MsgType::SerialBreak) => {
                    trace!("serial break");
                    serial_tx.send(MsgType::SerialBreak).unwrap();
                }
                Ok(MsgType::Serial(ch)) => {
                    trace!("serial: {ch:#02x}");
                    console_tx.send(MsgType::Console(ch)).unwrap();
                    if server {
                        net_clients.iter().for_each(|elem| elem.tx.send(MsgType::Console(ch)).unwrap());
                    }
                    if script_pid.load(Ordering::Relaxed) != 0 {
                        script_tx.send(MsgType::Console(ch)).unwrap();
                    }
                }
                Ok(MsgType::Exit) => {
                    info!("exit");
                    if server {
                        info!("Send exit to net clients");
                        net_clients.iter().for_each(|elem| elem.tx.send(MsgType::Exit).unwrap());
                    } else {
                        info!("Send exit to console service");
                        console_tx.send(MsgType::Exit).unwrap();
                    }
                }
                Ok(MsgType::ScriptAlertResponse(ch)) => {
                    trace!("script alert response: {ch:#02x}");
                    script_tx.send(MsgType::ScriptAlertResponse(ch)).unwrap();
                }
                Ok(MsgType::ScriptDone) => {
                    info!("Send done to script client");
                    script_tx.send(MsgType::ScriptDone).unwrap();
                }
                Err(e) => {
                    error!("Error: {e:?}");
                    break;
                }
            }
        }
    });
    termswx
}
