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

use crate::common::TextLocation;

#[derive(Debug, Clone)]
pub enum CompilerError {
    NoSuchVisitor(String),
    EmptyContext,
    BadGrammarElement,
    BadLiteral(String),
    InvalidRange,
    UnexpectedExprType(String, String),
    SyntaxError(String),
}

#[derive(Debug, Clone)]
pub struct CompilerLocatableError {
    pub error: CompilerError,
    pub location: TextLocation,
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::NoSuchVisitor(e) => write!(f, "No such visitor: {e}"),
            CompilerError::EmptyContext => write!(f, "Empty context"),
            CompilerError::BadGrammarElement => write!(f, "Bad grammar element"),
            CompilerError::BadLiteral(e) => write!(f, "Bad literal: {e}"),
            CompilerError::InvalidRange => write!(f, "Invalid range"),
            CompilerError::UnexpectedExprType(expected, actual) => write!(
                f,
                "Unexpected expression type: expected {expected} but got {actual}"
            ),
            CompilerError::SyntaxError(e) => write!(f, "Syntax error: {e}"),
        }
    }
}

impl Display for CompilerLocatableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.location)
    }
}

pub type CompileResult<T> = Result<T, CompilerLocatableError>;
