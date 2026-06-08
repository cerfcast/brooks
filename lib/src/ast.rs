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

use brooks_macros::GrammarName;
use crate::grammar::GrammarNode;
pub trait AST {}

pub struct Mel {}
pub struct FunctionCall {}

/// A MEL Expression
pub enum Expr {
    FunctionCall(FunctionCall),
    BinaryInfixOperation
}

impl AST for Mel {}
impl AST for Expr {}
