/*  Copyright (C) 2012-2023 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::path::Path;

use anyhow::Result;
use clap::{arg, ArgAction, command};
use log::LevelFilter;
use simple_logger::SimpleLogger;

use crate::configuration::Configuration;
use crate::configuration::io::{from_file, from_stdin};

mod configuration;

fn main() -> Result<()> {
    let matches = command!()
        .args(&[
            arg!(-i --input <FILE> "Path of the event file")
                .default_value("commands.json")
                .hide_default_value(false),
            arg!(-o --output <FILE> "Path of the result file")
                .default_value("compile_commands.json")
                .hide_default_value(false),
            arg!(-c --config <FILE> "Path of the config file"),
            arg!(-a --append "Append result to an existing output file")
                .action(ArgAction::SetTrue),
            arg!(-v --verbose ... "Sets the level of verbosity")
                .action(ArgAction::Count),
        ])
        .get_matches();

    // configure logging
    let verbose = matches.get_count("verbose");
    SimpleLogger::new()
        .with_level(into_log_level(verbose))
        .init()
        .unwrap();

    // read config
    let config = match matches.get_one::<String>("config").map(|s| s.as_ref()) {
        Some("-") =>
            from_stdin()?,
        Some(file) =>
            from_file(Path::new(file))?,
        None =>
            Configuration::default(),
    };

    println!("Hello, world! {:?}", config);

    log::trace!("trace message");
    log::debug!("debug message");
    log::info!("info message");
    log::warn!("warn message");
    log::error!("error message");

    Ok(())
}

fn into_log_level(count: u8) -> LevelFilter {
    match count {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}
