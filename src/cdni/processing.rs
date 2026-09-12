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

use std::error::Error;
use std::fmt::Debug;

use http::{Request, Response};

use crate::cdni::spec::TypedGenericMetadata;

#[derive(Debug)]
pub enum MetadataProcessingAnalysisError {
    InvalidType(String /* expected */, String /* actual */),
    InvalidMetadata(Box<dyn Error>),
    InvalidInput,
}

#[derive(Debug)]
pub enum MetadataProcessingInterpreterError {
    RuntimeError(Box<dyn Error>),
}

/// The context for the analysis of a given piece of Metadata Information.
///
/// Mostly here for future expansion.
#[derive(Debug)]
pub struct MetadataProcessingAnalysisContext {}

/// The context for the interpretation of a validated representation of a given piece of Metadata Information.
///
/// TODO: Determine whether the runtime can be factored out (made generic).
#[derive(Debug)]
pub struct MetadataProcessingInterpreterContext {
    pub request: Option<Request<Vec<u8>>>,
    pub response: Option<Response<Vec<u8>>>,
    pub runtime: tokio::runtime::Runtime,
}

pub type MetadataProcessingAnalysisResult = Result<
    (
        MetadataProcessingAnalysisContext,
        Box<dyn MetadataProcessingAnalyzed>,
    ),
    MetadataProcessingAnalysisError,
>;

pub type MetadataProcessingInterpretResult =
    Result<MetadataProcessingInterpreterContext, MetadataProcessingInterpreterError>;

/// Interpreter a validated representation of a given piece of Metadata Information.
pub trait MetadataProcessingAnalyzed: Debug {
    fn interpret(
        &self,
        input: MetadataProcessingInterpreterContext,
    ) -> MetadataProcessingInterpretResult;
}

/// Generate a validated representation of a given piece of Metadata Information, if possible.
pub trait MetadataProcessingAnalyzer: Debug {
    fn analyze(
        &self,
        v: &TypedGenericMetadata<()>,
        input: MetadataProcessingAnalysisContext,
    ) -> MetadataProcessingAnalysisResult;
}
