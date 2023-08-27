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

extern crate core;

use std::fs::OpenOptions;
use std::io::stdin;
use std::thread;

use anyhow::{anyhow, Context, Result};
use clap::{arg, ArgAction, ArgMatches, command};
use crossbeam_channel::{bounded, Sender, unbounded};
use json_compilation_db::{Entry, read, write};
use log::{error, LevelFilter};
use simple_logger::SimpleLogger;

use crate::configuration::Configuration;
use crate::configuration::io::from_reader;
use crate::execution::Execution;
use crate::filter::EntryPredicate;

mod configuration;
mod events;
mod execution;
mod compilation;
mod tools;
mod filter;
mod fixtures;


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
    configure_logging(&matches)
        .context("Configure logging from command line arguments.")?;

    // check semantic of the arguments
    let input = matches.get_one::<String>("input")
        .map(String::as_str)
        .expect("input is defaulted");
    let output = matches.get_one::<String>("output")
        .map(String::as_str)
        .expect("output is defaulted");
    let config = matches.get_one::<String>("config")
        .map(String::as_str);
    let append = matches.get_one::<bool>("append")
        .unwrap_or(&false);

    if input == "-" && config.unwrap_or("+") == "-" {
        error!("Both input and config reading the standard input.");
        return Err(anyhow!("Both input and config reading the standard input."));
    }
    if *append && output == "-" {
        error!("Append can't applied to the standard output.");
        return Err(anyhow!("Append can't applied to the standard output."));
    }

    // read configuration
    let config = match config {
        Some("-") => {
            let reader = stdin();
            from_reader(reader).context("Failed to read configuration from stdin")?
        }
        Some(file) => {
            let reader = OpenOptions::new().read(true).open(file)?;
            from_reader(reader)
                .with_context(|| format!("Failed to read configuration from file: {}", file))?
        }
        None =>
            Configuration::default(),
    };

    run(config, input.into(), output.into(), *append)
}

fn run(config: Configuration, input: String, output: String, append: bool) -> Result<()> {
    let (snd, rcv) = bounded::<Entry>(100);

    let captured_output = output.to_owned();
    thread::spawn(move || {
        new_entries_from_events(&snd, input.as_str()).expect("");
        if append {
            old_entries_from_previous_run(&snd, captured_output.as_str()).expect("");
        }
        drop(snd);
    });

    // consume the entry streams here
    let temp = format!("{}.tmp", &output);
    {
        let filter: EntryPredicate = config.output.content.into();
        let file = OpenOptions::new().write(true).open(&temp)?;
        write(file, rcv.iter().filter(filter))?;
    }
    std::fs::remove_file(&output)?;
    std::fs::rename(&output, &temp)?;

    Ok(())
}

fn old_entries_from_previous_run(sink: &Sender<Entry>, source: &str) -> Result<()> {
    let mut count: u32 = 0;
    let reader = OpenOptions::new().read(true).open(source)?;
    let events = read(reader);
    for event in events {
        match event {
            Ok(value) => {
                sink.send(value)?;
                count += 1;
            }
            Err(error) => {
                // todo
                log::error!("")
            }
        }
    }

    log::debug!("Found {count} entries from previous run.");
    Ok(())
}

fn new_entries_from_events(sink: &Sender<Entry>, input: &str) -> Result<u32> {
    let (exec_snd, exec_rcv) = unbounded::<Execution>();

    // log::debug!("Found {new_entries} entries");

    Ok(0)
}

fn configure_logging(matches: &ArgMatches) -> Result<()> {
    let level = match matches.get_count("verbose") {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    let mut logger = SimpleLogger::new()
        .with_level(level);
    if level <= LevelFilter::Debug {
        logger = logger.with_local_timestamps()
    }
    logger.init()?;

    Ok(())
}
