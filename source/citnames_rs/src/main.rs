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

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, stdin, stdout};
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::{anyhow, Context, Result};
use clap::{arg, ArgAction, command};
use crossbeam_channel::{bounded, Sender};
use json_compilation_db::Entry;
use log::LevelFilter;
use simple_logger::SimpleLogger;

use crate::configuration::{Compilation, Configuration};
use crate::execution::Execution;
use crate::filter::EntryPredicate;
use crate::tools::{RecognitionResult, Semantic, Tool};

mod configuration;
mod events;
mod execution;
mod compilation;
mod tools;
mod filter;
mod fixtures;

fn main() -> Result<()> {
    let arguments = Arguments::parse().validate()?;
    let application = Application::configure(arguments)?;
    application.run()?;

    Ok(())
}

#[derive(Debug, PartialEq)]
struct Arguments {
    input: String,
    output: String,
    config: Option<String>,
    append: bool,
    verbose: u8,
}

impl Arguments {
    fn parse() -> Self {
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

        Arguments {
            input: matches.get_one::<String>("input")
                .expect("input is defaulted")
                .clone(),
            output: matches.get_one::<String>("output")
                .expect("output is defaulted")
                .clone(),
            config: matches.get_one::<String>("config")
                .map(String::to_string),
            append: matches.get_one::<bool>("append")
                .unwrap_or(&false)
                .clone(),
            verbose: matches.get_count("verbose"),
        }
    }

    fn validate(self) -> Result<Self> {
        if self.input == "-" && self.config.as_deref() == Some("-") {
            return Err(anyhow!("Both input and config reading the standard input."));
        }
        if self.append && self.output == "-" {
            return Err(anyhow!("Append can't applied to the standard output."));
        }

        Ok(self)
    }

    fn prepare_logging(&self) -> Result<()> {
        let level = match &self.verbose {
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

    fn configuration(&self) -> Result<Configuration> {
        let configuration = match self.config.as_deref() {
            Some("-") | Some("/dev/stdin") => {
                let reader = stdin();
                serde_json::from_reader(reader)
                    .context("Failed to read configuration from stdin")?
            }
            Some(file) => {
                let reader = OpenOptions::new().read(true).open(file)?;
                serde_json::from_reader(reader)
                    .with_context(|| format!("Failed to read configuration from file: {}", file))?
            }
            None =>
                Configuration::default(),
        };
        Ok(configuration)
    }
}

#[derive(Debug, PartialEq)]
struct Application {
    arguments: Arguments,
    configuration: Configuration,
}

impl Application {
    fn configure(arguments: Arguments) -> Result<Self> {
        arguments.prepare_logging()?;

        let configuration = arguments.configuration()?;

        Ok(Application { arguments, configuration })
    }

    fn run(self) -> Result<()> {
        let (snd, rcv) = bounded::<Entry>(32);

        // Start reading entries (in a new thread), and send them across the channel.
        let (compilation_config, output_config) =
            (self.configuration.compilation, self.configuration.output);
        let output = PathBuf::from(&self.arguments.output);
        thread::spawn(move || {
            process_executions(self.arguments.input.as_str(), &compilation_config, &snd)
                .expect("Failed to process events.");

            if self.arguments.append {
                copy_entries(output.as_path(), &snd)
                    .expect("Failed to process existing compilation database");
            }
            drop(snd);
        });

        // Start writing the entries (from the channel) to the output.
        let filter: EntryPredicate = output_config.content.into();
        let entries = rcv.iter()
            .inspect(|entry| log::debug!("{:?}", entry))
            .filter(filter);
        match self.arguments.output.as_str() {
            "-" | "/dev/stdout" =>
                json_compilation_db::write(stdout(), entries)?,
            output => {
                let temp = format!("{}.tmp", output);
                // Create scope for the file, so it will be closed when the scope is over.
                {
                    let file = File::create(&temp)
                        .with_context(|| format!("Failed to create file: {}", temp))?;
                    let buffer = BufWriter::new(file);
                    json_compilation_db::write(buffer, entries)?;
                }
                std::fs::rename(&temp, output)
                    .with_context(|| format!("Failed to rename file from '{}' to '{}'.", temp, output))?;
            }
        };

        Ok(())
    }
}

fn copy_entries(source: &Path, destination: &Sender<Entry>) -> Result<()> {
    let mut count: u32 = 0;

    let file = OpenOptions::new().read(true).open(source)
        .with_context(|| format!("Failed to open file: {:?}", source))?;
    let buffer = BufReader::new(file);

    for event in json_compilation_db::read(buffer) {
        match event {
            Ok(value) => {
                destination.send(value)?;
                count += 1;
            }
            Err(_error) => {
                // todo
                log::error!("")
            }
        }
    }

    log::debug!("Found {count} entries from previous run.");
    Ok(())
}

fn process_executions(source: &str, config: &Compilation, destination: &Sender<Entry>) -> Result<()> {
    let (snd, rcv) = bounded::<Execution>(128);

    // Start worker threads, which will process executions and create compilation database entry.
    for _ in 0..num_cpus::get() {
        let tool: Box<dyn Tool> = config.into();
        let captured_sink = destination.clone();
        let captured_source = rcv.clone();
        thread::spawn(move || {
            for execution in captured_source.into_iter() {
                let result = tool.recognize(&execution);
                match result {
                    RecognitionResult::Recognized(Ok(Semantic::Compiler(call))) => {
                        log::debug!("execution recognized as compiler call, {:?} : {:?}", call, execution);
                        let entries: Result<Vec<Entry>> = call.try_into();
                        match entries {
                            Ok(entries) => for entry in entries {
                                captured_sink.send(entry).expect("")
                            }
                            Err(error) =>
                                log::debug!("can't convert into compilation entry: {}", error),
                        }
                    }
                    RecognitionResult::Recognized(Ok(_)) =>
                        log::debug!("execution recognized: {:?}", execution),
                    RecognitionResult::Recognized(Err(reason)) =>
                        log::debug!("execution recognized with failure, {:?} : {:?}", reason, execution),
                    RecognitionResult::NotRecognized =>
                        log::debug!("execution not recognized: {:?}", execution),
                }
            }
        });
    }

    // Start sending execution events from the given file.
    let buffer: BufReader<Box<dyn std::io::Read>> = match source {
        "-" | "/dev/stdin" =>
            BufReader::new(Box::new(stdin())),
        _ => {
            let file = OpenOptions::new().read(true).open(source)
                .with_context(|| format!("Failed to open file: {}", source))?;
            BufReader::new(Box::new(file))
        }
    };

    for execution in events::from_reader(buffer) {
        match execution {
            Ok(value) => {
                snd.send(value)?;
            }
            Err(_error) => {
                // todo
                log::error!("")
            }
        }
    }
    drop(snd);

    Ok(())
}
