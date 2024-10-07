//Steen Hegelund
//Time-Stamp: 2024-Oct-07 15:32
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Send and Receive bytes to/from the serial device and the TermSwitch

use std::io;
use std::thread;
use std::time::Duration;
use std::path::PathBuf;
use std::time::Instant;
use log::{error, trace};
use serialport::SerialPort;
use crossbeam_channel::{Sender, Receiver};
use crossterm::style::{Color, Stylize};

use crate::term_switch::{TermSwitch, MsgType};


fn run_serial(swi_tx: &Sender<MsgType>, ser_rx: &Receiver<MsgType>, start: Instant, prt: &Box<dyn SerialPort>) -> thread::JoinHandle<()> {
    let mut txport = prt.try_clone().unwrap();
    let mut rxport = prt.try_clone().unwrap();
    let switch_tx = swi_tx.clone();
    let serial_rx = ser_rx.clone();

    thread::spawn(move || {
        trace!("Wait for console input");
        let mut buffer = vec![0; 80];
        loop {
            match serial_rx.recv() {
                Ok(MsgType::Serial(val)) => {
                    buffer.clear();
                    buffer.push(val);
                    trace!("request: {val:#02x} {}", val as char);
                    match txport.write(&buffer) {
                        Ok(_) => (),
                        Err(e) => {
                            error!("Serial Write Error: {:?}", e);
                            crate::terminate(start, "Could not write to serial port");
                        }
                    }
                }
                Ok(MsgType::SerialBreak) => {
                    match txport.set_break() {
                        Ok(_) => {
                            thread::sleep(Duration::from_millis(100));
                            txport.clear_break().unwrap();
                        }
                        Err(_) => {
                            println!("\rNo break support on this serial interface\r");
                        }
                    }
                }
                Ok(MsgType::SerialClose) => {
                    trace!("Serial Close");
                    break;
                }
                Ok(_) => (),
                Err(e) => {
                    error!("Error: {e:?}");
                    break;
                }
            }
        }
    });

    let handle = thread::spawn(move || {
        trace!("Wait for serial input");
        loop {
            let mut serial_buf = vec![0; 1024];
            match rxport.read(serial_buf.as_mut_slice()) {
                Ok(cnt) => {
                    trace!("Received: {cnt}");
                    for idx in 0..cnt {
                        let val: u8 = serial_buf[idx];
                        trace!("Send: {val:#02x} '{}''", val as char);
                        let msg = MsgType::Serial(val);
                        switch_tx.send(msg).unwrap();
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => {
                    trace!("Serial Recv Error: {e:?}");
                    switch_tx.send(MsgType::SerialClose).unwrap();
                    break;
                }
            }
        }
    });
    handle
}

fn show_device_disconnect(portname: &str) {
    let msg = format!("Disconnected from: {:?}", portname);
    trace!("{}", msg);
    println!("\r{}\r", msg.with(Color::White).on(Color::DarkRed));
}

fn show_device_connect(port: &Box<dyn SerialPort>) {
    let msg = format!("Connected to: {:?} - baudrate: {:?}", port.name().unwrap(), port.baud_rate().unwrap());
    trace!("{}", msg);
    println!("\r{}\r", msg.with(Color::White).on(Color::DarkBlue));
}


fn do_open(portname: &str, baudrate: u32) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(portname, baudrate).timeout(Duration::from_millis(100)).open()
}


// Open the serial device and pass characters
pub fn open_device(termswx: &TermSwitch, device: PathBuf, baudrate: u32, keeprunning: bool, start: Instant) {
    let switch_tx = termswx.get_switch_tx();
    let serial_rx = termswx.get_serial_rx();

    thread::spawn(move || {
        let portname = device.to_str().unwrap();
        let mut running = true;
        loop {
            match do_open(portname, baudrate) {
                Ok(port) => {
                    running = true;
                    show_device_connect(&port);
                    let handle = run_serial(&switch_tx, &serial_rx, start, &port);
                    handle.join().unwrap();
                }
                Err(_) => {
                    if !keeprunning {
                        let msg = format!("The {} port has disappeared", portname);
                        crate::terminate(start, &msg);
                    } else {
                        if running {
                            show_device_disconnect(portname);
                        } else {
                            thread::sleep(Duration::from_secs(1));
                        }
                        running = false;
                    }
                }
            }
        }
    });
}
