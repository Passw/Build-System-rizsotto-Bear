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

use std::path::{Path, PathBuf};
use json_compilation_db::Entry;
use thiserror::Error;
use path_absolutize::Absolutize;

/// Represents an executed command semantic.
#[derive(Debug, PartialEq)]
enum Semantic {
    Unknown,
    Compiler(CompilerCall),
}

/// Represents a compiler call.
#[derive(Debug, PartialEq)]
enum CompilerCall {
    Query,
    Preprocess,
    Compile {
        working_dir: PathBuf,
        compiler: PathBuf,
        flags: Vec<String>,
        sources: Vec<PathBuf>,
        output: Option<PathBuf>,
    },
}

#[derive(Error, Debug)]
enum Error {
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("encode error")]
    OsString,
}

impl TryFrom<CompilerCall> for Vec<Entry> {
    type Error = Error;

    fn try_from(value: CompilerCall) -> Result<Self, Self::Error> {
        match value {
            CompilerCall::Compile { working_dir, compiler, flags, sources, output } => {
                sources.iter()
                    .map(|source| -> Result<Entry, Self::Error> {
                        let mut arguments: Vec<String> = vec![];
                        // Assemble the arguments as it would be for a single source file.
                        arguments.push(into_string(&compiler)?);
                        for flag in &flags {
                            arguments.push(flag.clone());
                        }
                        if let Some(file) = &output {
                            arguments.push(String::from("-o"));
                            arguments.push(into_string(file)?)
                        }
                        arguments.push(into_string(source)?);

                        Ok(
                            Entry {
                                file: into_abspath(source.clone(), working_dir.as_path())?,
                                directory: working_dir.clone(),
                                output: into_abspath_opt(output.clone(), working_dir.as_path())?,
                                arguments: arguments.clone(),
                            }
                        )
                    })
                    .collect()
            }
            _ =>
                Ok(vec![]),
        }
    }
}

fn into_abspath(path: PathBuf, root: &Path) -> Result<PathBuf, std::io::Error> {
    let candidate = if path.is_absolute() {
        path.absolutize()
    } else {
        path.absolutize_from(root)
    };
    candidate.map(|x| x.to_path_buf())
}

fn into_abspath_opt(path: Option<PathBuf>, root: &Path) -> Result<Option<PathBuf>, std::io::Error> {
    path.map(|v| into_abspath(v, root))
        .map_or(Ok(None), |v| v.map(Some))
}

fn into_string(path: &PathBuf) -> Result<String, Error> {
    path.clone().into_os_string().into_string().map_err(|_| Error::OsString)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_non_compilations() -> Result<(), Error> {
        let empty: Vec<Entry> = vec![];

        let result: Vec<Entry> = CompilerCall::Query.try_into()?;
        assert_eq!(empty, result);

        let result: Vec<Entry> = CompilerCall::Preprocess.try_into()?;
        assert_eq!(empty, result);

        Ok(())
    }

    #[test]
    fn test_single_source_compilation() -> Result<(), Error> {
        let input = CompilerCall::Compile {
            working_dir: PathBuf::from("/home/user"),
            compiler: PathBuf::from("clang"),
            flags: vec![String::from("-Wall")],
            sources: vec![PathBuf::from("source.c")],
            output: Some(PathBuf::from("source.o")),
        };

        let expected = vec![
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/home/user/source.c"),
                arguments: vec!["clang", "-Wall", "-o", "source.o", "source.c"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                output: Some(PathBuf::from("/home/user/source.o")),
            }
        ];

        let result: Vec<Entry> = input.try_into()?;

        assert_eq!(expected, result);

        Ok(())
    }

    #[test]
    fn test_multiple_sources_compilation() -> Result<(), Error> {
        let input = CompilerCall::Compile {
            working_dir: PathBuf::from("/home/user"),
            compiler: PathBuf::from("clang"),
            flags: vec![String::from("-Wall")],
            sources: vec![PathBuf::from("/tmp/source1.c"), PathBuf::from("../source2.c")],
            output: None,
        };

        let expected = vec![
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/tmp/source1.c"),
                arguments: vec!["clang", "-Wall", "/tmp/source1.c"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                output: None,
            },
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/home/source2.c"),
                arguments: vec!["clang", "-Wall", "../source2.c"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                output: None
            }
        ];

        let result: Vec<Entry> = input.try_into()?;

        assert_eq!(expected, result);

        Ok(())
    }
}
