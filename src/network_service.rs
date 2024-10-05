//Steen Hegelund
//Time-Stamp: 2024-Oct-05 11:36
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Send and Receive via a TCP network connection.
// This provides both the server side and the client side of the operation

use crate::term_switch::{TermSwitch, MsgType};

use log::{error, trace};
use std::path::PathBuf;
use std::time::Instant;
use std::thread;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::prelude::*;
use crossbeam_channel::{Sender, Receiver};
use crossterm::style::{Color, Stylize};
use chrono;
use std::sync::atomic::Ordering;


// Open the client connection to a server port
pub fn open_connection(termswx: &mut TermSwitch, device: PathBuf, start: Instant) {
    let path = device.to_str().unwrap();
    trace!("open_connection {}", path);

    let serial_rx = termswx.get_serial_rx();

    match TcpStream::connect(&path) {
        Ok(mut stream) => {
            let mut stream_rx = stream.try_clone().unwrap();
            thread::spawn(move || {
                trace!("Wait for console input");
                let mut buffer = vec![0; 2];
                loop {
                    match serial_rx.recv() {
                        Ok(MsgType::Serial(val)) => {
                            buffer.clear();
                            buffer.push(val);
                            trace!("send: {:#02x} '{}'", val, val as char);
                            match stream.write(&buffer) {
                                Ok(_) => (),
                                Err(_) => {
                                    crate::terminate(start, "Client Write Error");
                                }
                            }
                            let _ = stream.flush();
                        }
                        Ok(MsgType::Exit) => {
                            trace!("Serial Exit received");
                            break;
                        }
                        Ok(_) => (),
                        Err(e) => {
                            error!("Error: {:?}", e);
                            break;
                        }
                    }
                }
            });

            let switch_tx = termswx.get_switch_tx();
            thread::spawn(move || {
                trace!("Wait for network input");
                let mut buffer = [0; 10];
                loop {
                    match stream_rx.read(&mut buffer) {
                        Ok(0) => {
                            crate::terminate(start, "Client Connection Closed");
                        }
                        Ok(cnt) => {
                            for idx in 0..cnt {
                                trace!("received: {:#02x} '{}'", buffer[idx], buffer[idx] as char);
                                let msg = MsgType::Serial(buffer[idx]);
                                switch_tx.send(msg).unwrap();
                            }
                        }
                        Err(e) => {
                            println!("Client Recv Error: {:?}", e);
                            crate::terminate(start, "Connect has been reset");
                        }
                    }
                }
            });
        }
        Err(e) => {
            error!("Error: {:?}", e);
            crate::terminate(start, "Could not connect to server");
        }
    }
}


fn serve_client(addr: SocketAddr, client_rx: Receiver<MsgType>, mut stream_rx: TcpStream, switch_tx: &Sender<MsgType>) {
    let mut stream_tx = stream_rx.try_clone().unwrap();
    let sw_tx = switch_tx.clone();

    thread::spawn(move || {
        let mut buffer = vec![0; 10];
        loop {
            match client_rx.recv() {
                Ok(MsgType::Console(val)) => {
                    buffer.clear();
                    buffer.push(val);
                    trace!("output: {val:#02x} {}", val as char);
                    let _cnt = stream_tx.write(&buffer);
                    let _ = stream_tx.flush();
                }
                Ok(MsgType::Exit) => {
                    trace!("Network Exit received");
                    sw_tx.send(MsgType::Remove(addr)).unwrap();
                    break;
                }
                Ok(_) => (),
                Err(e) => {
                    error!("Error: {:?}", e);
                    break;
                }
            }
        }
    });

    let sw_tx = switch_tx.clone();
    thread::spawn(move || {
        let mut buffer = [0; 10];
        loop {
            match stream_rx.read(&mut buffer) {
                Ok(0) => {
                    let now = chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let text = format!("Client connection closed at {} from {}", now, addr);
                    trace!("{}", text);
                    println!("\r\n{}", text.with(Color::White).on(Color::Black));
                    sw_tx.send(MsgType::NetClientExit(addr)).unwrap();
                    break;
                }
                Ok(cnt) => {
                    for idx in 0..cnt {
                        trace!("input: {:#02x} {}", buffer[idx], buffer[idx] as char);
                        let msg = MsgType::Console(buffer[idx]);
                        sw_tx.send(msg).unwrap();
                    }
                }
                Err(e) => {
                    println!("{}", format!("\r\nReceive Error: {:?}\r", e).with(Color::White).on(Color::Black));
                    trace!("Receive Error: Client connection closed, send exit");
                    sw_tx.send(MsgType::Exit).unwrap();
                    break;
                }
            }
        }
    });
}


// Start a TCP server on a port number
pub fn start_server(termswx: &mut TermSwitch, portnum: u16, maxclients: i8, start: Instant) {
    let path = format!("0.0.0.0:{}", portnum);
    trace!("start_server path: {}", path);

    let switch_tx = termswx.get_switch_tx();
    let network_rx = termswx.get_network_rx();
    let clients = termswx.get_clients();

    thread::spawn(move || {
        match TcpListener::bind(&path) {
            Ok(listener) => {
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream_rx) => {
                            let addr = stream_rx.peer_addr().unwrap();
                            let now = chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

                            if clients.load(Ordering::Relaxed) >= maxclients {
                                let text = format!("Client rejected at {} from {}: Maximum clients connected: {}",
                                    now, addr, maxclients);
                                trace!("{}", text);
                                println!("\r\n{}", text.with(Color::White).on(Color::Black));
                                continue;
                            }

                            let text = format!("\r\nClient connection established at {} from {}", now, addr);
                            trace!("{}", text);
                            println!("\r\n{}", text.with(Color::White).on(Color::Black));

                            // The thread must exit to close the network_rx channel
                            // or there will be multiple receivers fighting for the same events

                            // Get a client connection channel
                            switch_tx.send(MsgType::Add(addr)).unwrap();
                            match network_rx.recv() {
                                Ok(MsgType::Added(client_rx)) => {
                                    serve_client(addr, client_rx, stream_rx, &switch_tx);
                                }
                                Ok(_) => (),
                                Err(_) => (),
                            }
                        }
                        Err(e) => {
                            println!("Client connection error {:?}!\r\n", e);
                            error!("Error: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error: {:?}", e);
                crate::terminate(start, "Port number already in use");
            }
        }
    });
}

