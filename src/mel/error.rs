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

use crate::{
    common::TextLocation,
    mel::{compiler::error::CompilerLocatableError, types::Type},
};

#[derive(Debug, Clone)]
pub enum AnalysisAssertions {
    ContextMissing,
    ContextMissingExpr(String),
    ContextWrongExprType(String, String, String),
    ContextMissingParams,
    InvalidOperator(String, String),
}

impl Display for AnalysisAssertions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisAssertions::ContextMissing => write!(f, "Missing analysis context"),
            AnalysisAssertions::ContextWrongExprType(expected, actual, whre) => write!(
                f,
                "Wrong expression type in analysis context; expected {} but found {} in {}",
                expected, actual, whre
            ),
            AnalysisAssertions::ContextMissingParams => {
                write!(f, "Missing parameters in analysis context")
            }
            AnalysisAssertions::ContextMissingExpr(s) => {
                write!(f, "Missing expression in {}", s)
            }
            AnalysisAssertions::InvalidOperator(o, l) => {
                write!(f, "Invalid operator {o} in {l}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AnalysisPreconditions {
    ContextMissingExpr(String),
}

impl Display for AnalysisPreconditions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisPreconditions::ContextMissingExpr(s) => {
                write!(f, "Missing expression in {}", s)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AnalysisError {
    CompilerError(CompilerLocatableError),
    Mismatch(Type, Type),
    RegexSame,
    InvalidType(Vec<Type>, Type),
    Miscount(usize, usize),
    InvalidRegex(String),
    UnknownField(String, String),
    UnknownIdentifier(String),
    AssertionFailure(AnalysisAssertions),
    PreconditionFailure(AnalysisPreconditions),
    OptimizationNotSupported(String),
    Incalculable,
}

impl Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilerError(e) => write!(f, "Compiler error: {:?}", e),
            Self::Mismatch(expected, found) => {
                write!(f, "Expected {:?}, found {:?}", expected, found)
            }
            Self::Miscount(expected, found) => {
                write!(f, "Expected {expected} arguments, found {found}")
            }
            Self::UnknownIdentifier(i) => write!(f, "Unknown identifier {:?}", i),
            Self::AssertionFailure(c) => write!(f, "Missing compiler context: {:?}", c),
            Self::PreconditionFailure(c) => {
                write!(f, "Precondition not satisfied during analysis: {:?}", c)
            }
            Self::UnknownField(strct, member) => write!(f, "No field named {member} in {strct}"),
            Self::Incalculable => write!(f, "Incalculable expression analysis"),
            Self::OptimizationNotSupported(o) => write!(f, "Optimization not supported: {o}"),
            Self::InvalidRegex(i) => write!(f, "Regular expression literal not valid: {i}"),
            Self::RegexSame => write!(
                f,
                "The operands to the regular expression match operator must be a string and a regular expression"
            ),
            Self::InvalidType(valids, actual) => write!(
                f,
                "{actual} is not one of the expected types ({})",
                valids
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisLocatableError {
    pub error: Box<AnalysisError>,
    pub location: TextLocation,
}

impl Display for AnalysisLocatableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.location)
    }
}
