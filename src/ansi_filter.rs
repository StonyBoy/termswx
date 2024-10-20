//Steen Hegelund
//Time-Stamp: 2024-Oct-17 20:55
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Handle ANSI Escape Sequences from a terminal and filter them out of a byte stream
// If the sequence does not start with "ESC [" the bytes are not filtered


use log::trace;
use std::collections::VecDeque;


// State machine states
#[derive(Debug, PartialEq)]
enum FilterState {
    Init,
    Start,
    Mode,
    Screen,
    Paste,
}

// State data for the state machine
#[derive(Debug)]
pub struct AnsiFilter {
    state: FilterState,   // Current state
    buffer: VecDeque<u8>, // Collected bytes, will be dropped if this is a ANSI sequence
}

impl AnsiFilter {

    // Create a new filter statemachine in the init state
    pub fn new() -> AnsiFilter {
        AnsiFilter {
            state: FilterState::Init,
            buffer: VecDeque::new(),
        }
    }

    pub fn next(self: &mut Self) -> Option<u8> {
        if self.state == FilterState::Init {
            return self.buffer.pop_front();
        }
        None
    }

    pub fn input(self: &mut Self, val: u8) {
        let ch = val as char;
        trace!("filter input: {:#02x} '{}' in {:?}", val, ch, self.state);
        match self.state {
            // If an ESC is received then go to Start state
            FilterState::Init => {
                if val == 0x1b {
                    self.state = FilterState::Start;
                    self.buffer.push_back(val);
                } else {
                    self.buffer.push_back(val);
                }
            }
            // Handle ANSI '[' that starts a Control Sequence Introducer
            FilterState::Start => {
                if ch == '[' {
                    self.state = FilterState::Mode;
                    self.buffer.push_back(val);
                } else {
                    self.state = FilterState::Init;
                    self.buffer.push_back(val);
                }
            }
            // Handle numbers (Screen) or '?' which is a paste command
            FilterState::Mode => {
                if ch.is_digit(10) {
                    self.state = FilterState::Screen;
                    self.buffer.push_back(val);
                } else if ch == '?' {
                    self.state = FilterState::Paste;
                    self.buffer.push_back(val);
                } else {
                    self.state = FilterState::Init;
                    self.buffer.clear();
                }
            }
            // Handle screen related sequences.  These end with an alpha character
            FilterState::Screen => {
                if ch.is_digit(10) {
                    self.state = FilterState::Screen;
                    self.buffer.push_back(val);
                } else if ch.is_alphabetic() {
                    self.state = FilterState::Init;
                    self.buffer.clear();
                } else {
                    self.state = FilterState::Screen;
                    self.buffer.push_back(val);
                }
            }
            // Handle paste related sequences.  These end with non digit character
            FilterState::Paste => {
                if ch.is_digit(10) {
                    self.state = FilterState::Screen;
                    self.buffer.push_back(val);
                } else {
                    self.state = FilterState::Init;
                    self.buffer.clear();
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    // importing names from outer scope
    use super::*;

    #[test]
    fn pure_ascii() {
        let mut filter = AnsiFilter::new();
        let mut testdata: Vec<u8> = vec![0x41, 0x42, 0x43, 0x51, 0x52, 0x53];
        let expdata: Vec<Option<u8>> = vec![Some(0x41), Some(0x42), Some(0x43), Some(0x51), Some(0x52), Some(0x53)];

        for (pos, val) in testdata.iter_mut().enumerate() {
            filter.input(*val);
            assert_eq!(filter.next(), expdata[pos], "Input: {:#02x}", val);
        }
    }

    #[test]
    fn set_color_green() {
        let mut filter = AnsiFilter::new();
        let mut testdata: Vec<u8> = vec![0x41, 0x42, 0x43, 0x1b, 0x5b, 0x30, 0x3b, 0x33, 0x32, 0x6d, 0x51, 0x52, 0x53];
        let expdata: Vec<Option<u8>> = vec![Some(0x41), Some(0x42), Some(0x43), None, None, None, None, None, None, None, Some(0x51), Some(0x52), Some(0x53)];

        for (pos, val) in testdata.iter_mut().enumerate() {
            filter.input(*val);
            assert_eq!(filter.next(), expdata[pos], "Input: {:#02x}", val);
        }
    }

    #[test]
    fn reset() {
        let mut filter = AnsiFilter::new();
        let mut testdata: Vec<u8> = vec![0x41, 0x42, 0x43, 0x1b, 0x5b, 0x30, 0x6d, 0x51, 0x52, 0x53];
        let expdata: Vec<Option<u8>> = vec![Some(0x41), Some(0x42), Some(0x43), None, None, None, None, Some(0x51), Some(0x52), Some(0x53)];

        for (pos, val) in testdata.iter_mut().enumerate() {
            filter.input(*val);
            assert_eq!(filter.next(), expdata[pos], "Input: {:#02x}", val);
        }
    }

    #[test]
    fn single_escape() {
        let mut filter = AnsiFilter::new();
        let mut testdata: Vec<u8> = vec![0x41, 0x42, 0x43, 0x1b, 0x51, 0x52, 0x53, 0x00];
        let expdata: Vec<Option<u8>> = vec![Some(0x41), Some(0x42), Some(0x43), None, Some(0x1b), Some(0x51), Some(0x52), Some(0x53)];

        for (pos, val) in testdata.iter_mut().enumerate() {
            filter.input(*val);
            assert_eq!(filter.next(), expdata[pos], "Input: {:#02x}", val);
        }
    }

    #[test]
    fn not_ansi_seq() {
        let mut filter = AnsiFilter::new();
        let mut testdata: Vec<u8> = vec![0x41, 0x42, 0x43, 0x1b, 0x45, 0x30, 0x3b, 0x33, 0x32, 0x6d, 0x51, 0x52, 0x53];
        let expdata: Vec<Option<u8>> = vec![Some(0x41), Some(0x42), Some(0x43), None, Some(0x1b), Some(0x45), Some(0x30), Some(0x3b), Some(0x33), Some(0x32), Some(0x6d), Some(0x51), Some(0x52), Some(0x53)];

        for (pos, val) in testdata.iter_mut().enumerate() {
            filter.input(*val);
            assert_eq!(filter.next(), expdata[pos], "Input: {:#02x}", val);
        }
    }

}
