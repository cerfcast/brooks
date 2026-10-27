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

use crate::mel::{
    compiler::error::CompilerLocatableError as MelCompilerLocatableError,
    error::AnalysisLocatableError as MelAnalysisLocatableError, types::Type,
};

#[derive(Debug, Clone, Default)]
pub enum VerificationError {
    #[default]
    NoError,
    WrongType,
    WrongGenericMetadataTypeName(String, String),
    NoVerifiedValue,
    ExpressionCompile(Box<MelCompilerLocatableError>),
    ExpressionAnalyze(Box<MelAnalysisLocatableError>),
    ExpressionWrongType(Type, Type),
    InvalidTypedResponseStage,
    ParseError,
}

impl Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationError::NoError => write!(f, "No error!"),
            VerificationError::WrongType => write!(f, "Wrong type"),
            VerificationError::WrongGenericMetadataTypeName(expected, actual) => write!(
                f,
                "Wrong generic metadata type; expected {expected} and got {actual}"
            ),
            VerificationError::NoVerifiedValue => write!(f, "Missing verified value"),
            VerificationError::ExpressionCompile(compiler_error) => {
                write!(f, "Expression compilation error: {compiler_error}")
            }
            VerificationError::ExpressionAnalyze(mel_analysis_locatable_error) => write!(
                f,
                "Expression analysis error: {mel_analysis_locatable_error}"
            ),
            VerificationError::ExpressionWrongType(expected, actual) => write!(
                f,
                "Wrong expression type; expected {expected} and got {actual}"
            ),
            VerificationError::InvalidTypedResponseStage => {
                write!(f, "Invalid typed response stage")
            }
            VerificationError::ParseError => write!(f, "Parse error"),
        }
    }
}
