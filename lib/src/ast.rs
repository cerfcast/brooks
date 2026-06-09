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

use crate::grammar::GrammarNode;
use brooks_macros::grammar_name;

use std::fmt::Debug;
pub trait AST: Debug {}

#[derive(Debug)]
#[grammar_name(mel)]
pub struct Mel {}

#[derive(Debug)]
#[grammar_name(function_call_expr)]
pub struct FunctionCall {}

/// A MEL Expression
#[derive(Debug)]
#[grammar_name(expr)]
pub enum Expr {
    FunctionCall(FunctionCall),
    BinaryInfixOperation,
}

impl AST for Mel {}
impl AST for Expr {}
impl AST for FunctionCall {}
