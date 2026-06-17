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

use crate::grammar::{GrammarNode, GrammarLocation};
use brooks_macros::{grammar_location, grammar_name};
use std::fmt::Debug;

#[grammar_name(mel)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct Mel {
    pub testing: usize
}

#[grammar_name(function_call_expr)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub callee: Identifier,
    pub arguments: ArgumentList,
}

#[grammar_name(identifier)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct Identifier {
    pub identifier: String,
}

#[grammar_name(argument)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct Argument {
    pub expr: Expr,
}

#[grammar_name(argument_list)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct ArgumentList {
    pub arguments: Vec<Argument>,
}

#[grammar_name(binary_infix_operator)]
#[derive(Debug, Clone)]
pub enum BinaryInfixOperator {
    Logic(LogicOperator),
    Comparison(ComparisonOperator),
    Math(MathOperator),
    Concat(StringConcatOperator),
}

#[grammar_name(comparison_operator)]
#[derive(Debug, Clone)]
pub enum ComparisonOperator {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[grammar_name(logic_operator)]
#[derive(Debug, Clone)]
pub enum LogicOperator {
    And,
    Or,
}

#[grammar_name(string_concat)]
#[derive(Debug, Clone)]
pub struct StringConcatOperator {}

#[grammar_name(math_operator)]
#[derive(Debug, Clone)]
pub enum MathOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
}

#[grammar_name(binary_expr)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Expr,
    pub op: BinaryInfixOperator,
    pub right: Expr,
}

#[grammar_name(expr)]
#[derive(Debug, Clone)]
pub enum Expr {
    FunctionCall(Box<FunctionCall>),
    BinaryExpr(Box<BinaryExpr>),
    Identifier(Box<Identifier>),
    ArgumentList(Box<ArgumentList>),
    Argument(Box<Argument>),
    Literal(Box<Literal>, GrammarLocation),
}

impl Expr {
    pub fn location(&self) -> GrammarLocation {
        match self {
            Expr::FunctionCall(function_call) => function_call.location.clone(),
            Expr::BinaryExpr(binary_expr) => binary_expr.location.clone(),
            Expr::Identifier(identifier) => identifier.location.clone(),
            Expr::ArgumentList(argument_list) => argument_list.location.clone(),
            Expr::Argument(argument) => argument.location.clone(),
            Expr::Literal(_, location) => location.clone(),
        }
    }
}

#[grammar_name(literal)]
#[derive(Debug, Clone)]
pub enum Literal {
    Boolean(BooleanLiteral),
    Number(NumberLiteral),
    String(StringLiteral),
}

#[grammar_name(boolean_literal)]
#[derive(Debug, Clone)]
pub enum BooleanLiteral {
    True,
    False,
}

#[derive(Debug, Clone)]
#[grammar_name(number_literal)]
pub struct NumberLiteral {
    pub literal: usize,
}

#[derive(Debug, Clone)]
#[grammar_name(string_literal)]
pub struct StringLiteral {
    pub literal: String,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum Type {
    Boolean,
    Integer,
    String,
    Params(Vec<Type>),
    Function(Box<Type>, Vec<Type>),
    #[default]
    None,
}

#[derive(Debug, Clone)]
pub enum AstVisitorError {}

pub type AstVisitorResult<T, E> = Result<T, E>;

pub trait AstVisitor<T, E> {
    fn visit_function_call(
        &self,
        ast: FunctionCall,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_identifier(
        &self,
        ast: Identifier,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_argument_list(
        &self,
        ast: ArgumentList,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_argument(
        &self,
        ast: Argument,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_binary_expr(
        &self,
        ast: BinaryExpr,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_literal(
        &self,
        ast: (Literal, GrammarLocation),
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
}

pub struct AstVisitorDriver {}

impl AstVisitorDriver {
    #[allow(dead_code)]
    pub fn visit<T, E>(
        &self,
        node: Expr,
        visitor: &impl AstVisitor<T, E>,
        context: T,
    ) -> AstVisitorResult<T, E> {
        match node {
            Expr::Argument(argument) => visitor.visit_argument(*argument, context, self),
            Expr::ArgumentList(argument_list) => {
                visitor.visit_argument_list(*argument_list, context, self)
            }
            Expr::Identifier(identifier) => visitor.visit_identifier(*identifier, context, self),
            Expr::FunctionCall(function_call) => {
                visitor.visit_function_call(*function_call, context, self)
            }
            Expr::BinaryExpr(binary_expr) => visitor.visit_binary_expr(*binary_expr, context, self),
            Expr::Literal(literal, location) => visitor.visit_literal((*literal, location), context, self),
        }
    }
}
