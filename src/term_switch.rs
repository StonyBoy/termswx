//Steen Hegelund
//Time-Stamp: 2024-Oct-11 12:12
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :

use log::{error, trace, info};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};
use std::sync::{Arc, atomic::AtomicU32, atomic::AtomicBool, atomic::AtomicI8, atomic::Ordering};
use std::net::SocketAddr;

// Messages sent via channels between threads
#[derive(Debug,Clone)]
pub enum MsgType {
    Add(SocketAddr),
    Added(Receiver<MsgType>),
    Console(u8),
    Serial(u8),
    SerialClose,
    SerialBreak,
    ScriptAlertResponse(u8),
    ScriptDone,
    NetClientExit(SocketAddr),
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
    binary_mode: Arc<AtomicBool>,

    clients: Arc<AtomicI8>,

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
    pub fn get_binary_mode(&self) -> Arc::<AtomicBool> {
        self.binary_mode.clone()
    }
    pub fn get_clients(&self) -> Arc::<AtomicI8> {
        self.clients.clone()
    }
}


fn net_clients_send(net_clients: &mut Vec<NetClient>, prefix: &str, msg: MsgType, clients: Arc::<AtomicI8>) {
    info!("{}: {:?}", prefix, msg);
    let mut r_idx = net_clients.len();
    for (pos, elem)  in net_clients.iter().enumerate()  {
        match elem.tx.send(msg.clone()) {
            Ok(_) => (),
            Err(_) => {
                error!("{}: Client {} gone at pos: {}", prefix, elem.addr, pos);
                r_idx = pos;
            }
        }
    }
    if r_idx != net_clients.len() {
        net_clients.remove(r_idx);
        info!("{}: Client removed at pos: {}", prefix, r_idx);
        clients.store(net_clients.len().try_into().expect("Get number of clients"), Ordering::Relaxed);
    }
}


fn net_client_send(net_clients: &mut Vec<NetClient>, prefix: &str, msg: MsgType, addr: SocketAddr, clients: Arc::<AtomicI8>) {
    info!("{}: {}", prefix, addr);
    for (pos, elem)  in net_clients.iter().enumerate()  {
        if elem.addr == addr {
            match elem.tx.send(msg.clone()) {
                Ok(_) => {
                    info!("{}: Client {} removed at pos: {}", prefix, elem.addr, pos);
                    net_clients.remove(pos);
                    clients.store(net_clients.len().try_into().expect("Get number of clients"), Ordering::Relaxed);
                    return;
                },
                Err(_) => {
                    error!("{}: Client {} gone and removed at pos: {}", prefix, elem.addr, pos);
                    net_clients.remove(pos);
                    clients.store(net_clients.len().try_into().expect("Get number of clients"), Ordering::Relaxed);
                    return;
                }
            }
        }
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
        binary_mode: Arc::new(AtomicBool::new(false)),
        clients: Arc::new(AtomicI8::new(0)),
        stop: false,
    };

    let script_pid = termswx.script_pid.clone();
    let binary_mode = termswx.binary_mode.clone();
    let clients = termswx.clients.clone();

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

                    info!("Add: {}", addr);
                    clients.store(net_clients.len().try_into().expect("Get number of clients"), Ordering::Relaxed);
                    network_tx.send(MsgType::Added(rx)).unwrap();
                }
                Ok(MsgType::Added(_)) => (),
                Ok(MsgType::Console(ch)) => {
                    trace!("console: {:#02x} '{}'", ch, ch as char);
                    match serial_tx.send(MsgType::Serial(ch)) {
                        Ok(_) => (),
                        Err(_) => {
                            error!("Client connection is dead");
                        }
                    }
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
                    trace!("serial: {:#02x} '{}'", ch, ch as char);
                    if server {
                        net_clients_send(&mut net_clients, "Serial", MsgType::Console(ch), clients.clone());
                    }
                    if script_pid.load(Ordering::Relaxed) != 0 {
                        script_tx.send(MsgType::Console(ch)).unwrap();
                        if !binary_mode.load(Ordering::Relaxed) {
                            console_tx.send(MsgType::Console(ch)).unwrap();
                        }
                    } else {
                        console_tx.send(MsgType::Console(ch)).unwrap();
                    }
                }
                Ok(MsgType::NetClientExit(addr)) => {
                    net_client_send(&mut net_clients, "NetClientExit", MsgType::Exit, addr, clients.clone());
                }
                Ok(MsgType::Exit) => {
                    info!("exit");
                    if server {
                        net_clients_send(&mut net_clients, "Exit", MsgType::Exit, clients.clone());
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
