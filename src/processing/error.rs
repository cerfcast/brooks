// brooks, Copyright 2026, Will Hawkins
//
// This file is part of brooks.

// This file is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Interpret CDNI Interconnection Metadata

use std::fmt::Display;

use http::StatusCode;

use crate::logging::LogMsgs;

use crate::processing;

pub type ImdInterpretValue = (StatusCode, http::Response<Vec<u8>>, LogMsgs);
pub type ImdInterpretResultError = (Box<ImdInterpretError>, LogMsgs);
pub type ImdInterpretResult = Result<ImdInterpretValue, (Box<ImdInterpretError>, LogMsgs)>;

#[derive(Debug)]
pub enum ImdInterpretError {
    TransformError(Box<dyn std::error::Error>),
    UpstreamError(String),
    RuntimeError(Box<dyn std::error::Error>),
    MetadataProcessingError(Box<processing::interpret::Error>),
}

impl Display for ImdInterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Interconnection Metadata Interpretation Error: ")?;
        match self {
            ImdInterpretError::MetadataProcessingError(ps_interpret_error) => {
                write!(f, "{ps_interpret_error}")
            }
            ImdInterpretError::UpstreamError(ue) => {
                write!(f, "{ue}")
            }
            ImdInterpretError::RuntimeError(re) => {
                write!(f, "{re}")
            }
            ImdInterpretError::TransformError(te) => write!(f, "{te}"),
        }
    }
}
