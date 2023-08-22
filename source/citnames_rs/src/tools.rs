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

use crate::configuration::Compilation;
use crate::execution::Execution;
use crate::tools::configured::Configured;
use crate::tools::RecognitionResult::{NotRecognized, Recognized};
use crate::tools::wrapper::Wrapper;

mod configured;
mod wrapper;
mod matchers;

/// This abstraction is representing a tool which is known by us.
pub(crate) trait Tool {
    /// A tool has a potential to recognize a command execution and identify
    /// the semantic of that command.
    fn recognize(&self, _: &Execution) -> RecognitionResult;
}

#[derive(Debug, PartialEq)]
pub(crate) enum RecognitionResult {
    Recognized(Result<Semantic, String>),
    NotRecognized,
}

/// Represents an executed command semantic.
#[derive(Debug, PartialEq)]
pub(crate) enum Semantic {
    Compiler(CompilerCall),
    UnixCommand,
    BuildCommand,
}

/// Represents a compiler call.
#[derive(Debug, PartialEq)]
pub(crate) enum CompilerCall {
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


struct Any {
    tools: Vec<Box<dyn Tool>>,
}

impl Tool for Any {
    /// Any of the tool recognize the semantic, will be returned as result.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        for tool in &self.tools {
            match tool.recognize(x) {
                Recognized(result) =>
                    return Recognized(result),
                _ => continue,
            }
        }
        NotRecognized
    }
}


struct ExcludeOr {
    excludes: Vec<PathBuf>,
    or: Box<dyn Tool>,
}

impl Tool for ExcludeOr {
    /// Check if the executable is on the exclude list, return as not recognized.
    /// Otherwise delegate the recognition to the tool given.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        for exclude in &self.excludes {
            if &x.executable == exclude {
                return NotRecognized;
            }
        }
        return self.or.recognize(x);
    }
}

impl From<Compilation> for Box<dyn Tool> {
    fn from(value: Compilation) -> Self {
        let mut tools = vec![
            Box::new(Wrapper::new()) as Box<dyn Tool>,
        ];

        // The hinted tools should be the first to recognize.
        if !value.compilers_to_recognize.is_empty() {
            let configured = Configured::from(value.compilers_to_recognize);
            tools.insert(0, Box::new(configured))
        }
        // Excluded compiler check should be done before anything.
        if !value.compilers_to_exclude.is_empty() {
            return Box::new(
                ExcludeOr {
                    // exclude the executables are explicitly mentioned in the config file.
                    excludes: value.compilers_to_exclude,
                    or: Box::new(Any { tools }),
                }
            );
        }
        // Return the tools we configured.
        Box::new(Any { tools })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::tools::CompilerCall::Query;
    use crate::tools::test::MockTool::Recognize;

    use super::*;

    #[test]
    fn test_any_when_no_match() {
        let sut = Any {
            tools: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::NotRecognize),
            ]
        };

        let input = any_execution();

        match sut.recognize(&input) {
            NotRecognized => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_any_when_match() {
        let sut = Any {
            tools: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::Recognize),
                Box::new(MockTool::NotRecognize),
            ]
        };

        let input = any_execution();

        match sut.recognize(&input) {
            Recognized(Ok(_)) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_any_when_match_fails() {
        let sut = Any {
            tools: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::RecognizeFailed),
                Box::new(MockTool::Recognize),
                Box::new(MockTool::NotRecognize),
            ]
        };

        let input = any_execution();

        match sut.recognize(&input) {
            Recognized(Err(_)) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_exclude_when_match() {
        let sut = ExcludeOr {
            excludes: vec![PathBuf::from("/usr/bin/something")],
            or: Box::new(Recognize),
        };

        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec![],
            working_dir: PathBuf::new(),
            environment: HashMap::new(),
        };

        match sut.recognize(&input) {
            NotRecognized => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_exclude_when_no_match() {
        let sut = ExcludeOr {
            excludes: vec![PathBuf::from("/usr/bin/something")],
            or: Box::new(Recognize),
        };

        let input = any_execution();

        match sut.recognize(&input) {
            Recognized(Ok(_)) => assert!(true),
            _ => assert!(false)
        }
    }

    enum MockTool {
        Recognize,
        RecognizeFailed,
        NotRecognize,
    }

    impl Tool for MockTool {
        fn recognize(&self, _: &Execution) -> RecognitionResult {
            match self {
                MockTool::Recognize =>
                    Recognized(Ok(Semantic::Compiler(Query))),
                MockTool::RecognizeFailed =>
                    Recognized(Err(String::from("problem"))),
                MockTool::NotRecognize =>
                    NotRecognized,
            }
        }
    }

    fn any_execution() -> Execution {
        Execution {
            executable: PathBuf::new(),
            arguments: vec![],
            working_dir: PathBuf::new(),
            environment: HashMap::new(),
        }
    }
}
