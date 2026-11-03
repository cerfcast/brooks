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

use std::fmt::Display;

use http::StatusCode;

use crate::{dcdn::interpret::Error, logging::LogMsgs};

pub type ProxyValue = (StatusCode, http::Response<Vec<u8>>, LogMsgs);
pub type ProxyResult = Result<ProxyValue, (Box<ProxyError>, LogMsgs)>;

#[derive(Debug)]
pub enum ProxyError {
    SetupError(Box<dyn std::error::Error>),
    HmdsError(Box<dyn std::error::Error>),
    ImdiError(ImdInterpretError),
    NativeError(Box<dyn std::error::Error>),
}

impl Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error proxying to/from the native request: ")?;
        match &self {
            ProxyError::SetupError(error) => write!(f, "{error}"),
            ProxyError::HmdsError(error) => write!(f, "{error}"),
            ProxyError::ImdiError(imd_interpret_error) => write!(f, "{imd_interpret_error}"),
            ProxyError::NativeError(native_error) => write!(f, "{native_error}"),
        }
    }
}

pub type ImdInterpretValue = (StatusCode, http::Response<Vec<u8>>, LogMsgs);
pub type ImdInterpretResultError = (Box<ImdInterpretError>, LogMsgs);
pub type ImdInterpretResult = Result<ImdInterpretValue, (Box<ImdInterpretError>, LogMsgs)>;

#[derive(Debug)]
pub enum ImdInterpretError {
    TransformError(Box<dyn std::error::Error>),
    UpstreamError(String),
    RuntimeError(Box<dyn std::error::Error>),
    MetadataProcessingError(Box<Error>),
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

impl std::error::Error for ImdInterpretError {}
