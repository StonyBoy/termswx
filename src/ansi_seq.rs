//Steen Hegelund
//Time-Stamp: 2024-Mar-15 11:08
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Handle ANSI Escape Sequences from a terminal and filter them out of a byte stream


use log::trace;
use std::collections::VecDeque;


// State machine states
#[derive(Debug)]
enum AnsiState {
    Init,
    Start,
    Mode,
    Screen,
    Paste,
}

// State data for the state machine
#[derive(Debug)]
pub struct AnsiSeqState {
    state: AnsiState,  // Current state
    buffer: VecDeque<u8>, // Collected bytes, will be dropped if this is a ANSI sequence
}

impl AnsiSeqState {

    // Create a new statemachine in the init state
    pub fn new() -> AnsiSeqState {
        AnsiSeqState {
            state: AnsiState::Init,
            buffer: VecDeque::new()
        }
    }

    pub fn input(self: &mut Self, val: &mut u8) -> Option<u8> {
        let ch = *val as char;
        trace!("fsm input: {:#02x} '{}' in {:?}", val, ch, self.state);
        match self.state {
            // Init state will save the value, mark it as handled, and return it on the next call
            // If an ESC is received then go to Start
            AnsiState::Init => {
                if *val == 0 {
                    return None;
                }
                if *val == 0x1b {
                    self.state = AnsiState::Start;
                    self.buffer.push_back(*val);
                } else {
                    let res = Some(*val);
                    *val = 0;
                    return res;
                }
            }
            // Handle ANSI '[' that starts a Control Sequence Introducer
            AnsiState::Start => {
                if ch == '[' {
                    self.state = AnsiState::Mode;
                    self.buffer.push_back(*val);
                } else {
                    self.state = AnsiState::Init;
                }
            }
            // Handle numbers (Screen) or '?' which is a paste command
            AnsiState::Mode => {
                if ch.is_digit(10) {
                    self.state = AnsiState::Screen;
                    self.buffer.push_back(*val);
                } else if ch == '?' {
                    self.state = AnsiState::Paste;
                    self.buffer.push_back(*val);
                } else {
                    self.state = AnsiState::Init;
                }
            }
            // Handle screen related sequences.  These end with an alpha character
            AnsiState::Screen => {
                if ch.is_digit(10) {
                    self.state = AnsiState::Screen;
                    self.buffer.push_back(*val);
                } else if ch.is_alphabetic() {
                    self.state = AnsiState::Init;
                    self.buffer.clear();
                } else {
                    self.state = AnsiState::Screen;
                    self.buffer.push_back(*val);
                }
            }
            // Handle paste related sequences.  These end with non digit character
            AnsiState::Paste => {
                if ch.is_digit(10) {
                    self.state = AnsiState::Screen;
                    self.buffer.push_back(*val);
                } else {
                    self.state = AnsiState::Init;
                    self.buffer.clear();
                }
            }
        }
        None
    }
}
