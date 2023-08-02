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

use thiserror::Error;

use crate::configuration::Configuration;
use crate::execution::Execution;
use crate::semantic::Semantic;

mod any;
mod exclude_or;
mod configured;
mod wrapper;
mod matchers;

#[derive(Error, Debug, PartialEq)]
pub(crate) enum Error {
    #[error("Executable not recognized")]
    ExecutableFailure,
    #[error("Argument not recognized")]
    ArgumentFailure,
    #[error("Source file not found")]
    SourceNotFound,
}

#[derive(Debug, PartialEq)]
pub(crate) enum RecognitionResult {
    Recognized(Result<Semantic, Error>),
    NotRecognized,
}

trait Tool {
    fn recognize(&self, _: &Execution) -> RecognitionResult;
}

fn init_from(cfg: Configuration) -> Box<dyn Tool> {
    todo!()
}
