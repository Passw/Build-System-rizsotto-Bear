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

use std::path::PathBuf;
use lazy_static::lazy_static;

use crate::configuration::CompilerToRecognize;
use crate::execution::Execution;
use crate::tools::{CompilerCall, Semantic};
use crate::tools::{Any, RecognitionResult, Tool};
use crate::tools::matchers::source::looks_like_a_source_file;
use crate::tools::RecognitionResult::{NotRecognized, Recognized};

pub(crate) struct Configured {
    config: CompilerToRecognize,
}

impl Configured {
    pub(crate) fn new(config: &CompilerToRecognize) -> Box<dyn Tool> {
        Box::new(Configured { config: config.clone() })
    }

    pub(crate) fn from(configs: &[CompilerToRecognize]) -> Box<dyn Tool> {
        Any::new(configs.into_iter().map(Configured::new).collect())
    }
}

impl Tool for Configured {
    /// Any of the tool recognize the semantic, will be returned as result.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        if x.executable == self.config.executable {
            let mut flags = vec![];
            let mut sources = vec![];

            // find sources and filter out requested flags.
            for argument in x.arguments.iter().skip(1) {
                if self.config.flags_to_remove.contains(&argument) {
                    continue;
                } else if looks_like_a_source_file(argument.as_str()) {
                    sources.push(PathBuf::from(argument));
                } else {
                    flags.push(argument.clone());
                }
            }
            // extend flags with requested flags.
            for flag in &self.config.flags_to_add {
                flags.push(flag.clone());
            }

            if sources.is_empty() {
                Recognized(Err(String::from("source file is not found")))
            } else {
                Recognized(
                    Ok(
                        Semantic::Compiler(
                            CompilerCall::Compile {
                                working_dir: x.working_dir.clone(),
                                compiler: x.executable.clone(),
                                flags,
                                sources,
                                output: None,
                            }
                        )
                    )
                )
            }
        } else {
            NotRecognized
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use crate::tools::Semantic::Compiler;

    use super::*;

    #[test]
    fn test_matching() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec!["something", "-Dthis=that", "-I.", "source.c", "-o", "source.c.o"]
                .iter()
                .map(|i| i.to_string())
                .collect(),
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        let expected = CompilerCall::Compile {
            working_dir: PathBuf::from("/home/user"),
            compiler: PathBuf::from("/usr/bin/something"),
            flags: vec!["-Dthis=that", "-o", "source.c.o", "-Wall"]
                .iter()
                .map(|i| i.to_string())
                .collect(),
            sources: vec![PathBuf::from("source.c")],
            output: None,
        };

        assert_eq!(Recognized(Ok(Compiler(expected))), SUT.recognize(&input));
    }

    #[test]
    fn test_matching_without_sources() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec!["something", "--help"]
                .iter()
                .map(|i| i.to_string())
                .collect(),
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(Recognized(Err(String::from("source file is not found"))), SUT.recognize(&input));
    }

    #[test]
    fn test_not_matching() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/cc"),
            arguments: vec!["cc", "-Dthis=that", "-I.", "source.c", "-o", "source.c.o"]
                .iter()
                .map(|i| i.to_string())
                .collect(),
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(NotRecognized, SUT.recognize(&input));
    }

    lazy_static! {
        static ref SUT: Configured = Configured {
            config: CompilerToRecognize {
                executable: PathBuf::from("/usr/bin/something"),
                flags_to_remove: vec![String::from("-I.")],
                flags_to_add: vec![String::from("-Wall")],
            }
        };
    }
}
