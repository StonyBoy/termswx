//Steen Hegelund
//Time-Stamp: 2024-Mar-15 10:59
//vim: set ts=4 sw=4 sts=4 tw=99 cc=120 et ft=rust :
//
// Log program trace using log4rs
// Log Message Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html


use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};


// Level Filter Values mapped by verbose
// Off: Default value
// error (highest priority): -v
// warn: -vv
// info: -vvv
// debug: -vvvv
// trace: -vvvvv
fn to_levelfilter(loglevel: usize) -> LevelFilter {
    match loglevel {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Off,
    }
}


// Logging to log file with a level filter specified by verbose value
pub fn init(file_path: String, verbose: usize) {
    let filter = to_levelfilter(verbose);
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)}: {M} - {l} - {m}\n")))
        .build(file_path)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(filter),) .unwrap();

    // Use this to change log levels at runtime.
    let _ = log4rs::init_config(config).unwrap();
}
