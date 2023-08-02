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

use crate::execution::Execution;
use crate::tools::{Error, RecognitionResult, Tool};
use crate::tools::RecognitionResult::{NotRecognized, Recognized};

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

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::semantic::CompilerCall::Query;
    use crate::semantic::Semantic;

    use super::*;

    #[test]
    fn test_when_no_match() {
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
    fn test_when_match() {
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
    fn test_when_match_fails() {
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
                    Recognized(Err(Error::ExecutableFailure)),
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
