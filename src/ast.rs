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

use crate::grammar::{GrammarLocation, GrammarNode};
use brooks_macros::{grammar_location, grammar_name};
use std::{fmt::Debug, sync::Arc};

#[grammar_name(mel)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct Mel<A: Debug + Clone> {
    pub testing: usize,
    pub aug: A,
}

#[grammar_name(function_call_expr)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct FunctionCall<A: Debug + Clone> {
    pub callee: Identifier<A>,
    pub arguments: ArgumentList<A>,
    pub aug: A,
}

#[grammar_name(identifier)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct Identifier<A: Debug + Clone> {
    pub identifier: String,
    pub aug: A,
}

#[grammar_name(argument)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct Argument<A: Debug + Clone> {
    pub expr: Expr<A>,
    pub aug: A,
}

#[grammar_name(argument_list)]
#[grammar_location]
#[derive(Debug, Clone)]
pub struct ArgumentList<A: Debug + Clone> {
    pub arguments: Vec<Argument<A>>,
    pub aug: A,
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
pub struct BinaryExpr<A: Debug + Clone> {
    pub left: Expr<A>,
    pub op: BinaryInfixOperator,
    pub right: Expr<A>,
    pub aug: A,
}

#[grammar_name(expr)]
#[derive(Debug, Clone)]
pub enum Expr<A: Debug + Clone> {
    FunctionCall(Arc<FunctionCall<A>>),
    BinaryExpr(Arc<BinaryExpr<A>>),
    Identifier(Arc<Identifier<A>>),
    ArgumentList(Arc<ArgumentList<A>>),
    Argument(Arc<Argument<A>>),
    Literal(Literal, GrammarLocation, A),
}

impl<A: Debug + Clone> Expr<A> {
    pub fn location(&self) -> GrammarLocation {
        match self {
            Expr::FunctionCall(function_call) => function_call.location.clone(),
            Expr::BinaryExpr(binary_expr) => binary_expr.location.clone(),
            Expr::Identifier(identifier) => identifier.location.clone(),
            Expr::ArgumentList(argument_list) => argument_list.location.clone(),
            Expr::Argument(argument) => argument.location.clone(),
            Expr::Literal(_, location, _) => location.clone(),
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
#[derive(Debug, Clone, PartialEq)]
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
    Function(Arc<Type>, Vec<Type>),
    #[default]
    None,
}

#[derive(Debug, Clone)]
pub enum AstVisitorError {}

pub type AstVisitorResult<T, E> = Result<T, E>;

pub trait AstVisitor<T, A: Debug + Clone, E> {
    fn visit_function_call(
        &self,
        ast: &FunctionCall<A>,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_identifier(
        &self,
        ast: &Identifier<A>,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_argument_list(
        &self,
        ast: &ArgumentList<A>,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_argument(
        &self,
        ast: &Argument<A>,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_binary_expr(
        &self,
        ast: &BinaryExpr<A>,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
    fn visit_literal(
        &self,
        ast: (&Literal, &GrammarLocation, &A),
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T, E>;
}

pub struct AstVisitorDriver {}

impl AstVisitorDriver {
    #[allow(dead_code)]
    pub fn visit<T, A: Debug + Clone, E>(
        &self,
        node: &Expr<A>,
        visitor: &impl AstVisitor<T, A, E>,
        context: T,
    ) -> AstVisitorResult<T, E> {
        match node {
            Expr::Argument(argument) => visitor.visit_argument(argument, context, self),
            Expr::ArgumentList(argument_list) => {
                visitor.visit_argument_list(argument_list, context, self)
            }
            Expr::Identifier(identifier) => visitor.visit_identifier(identifier, context, self),
            Expr::FunctionCall(function_call) => {
                visitor.visit_function_call(function_call, context, self)
            }
            Expr::BinaryExpr(binary_expr) => visitor.visit_binary_expr(binary_expr, context, self),
            Expr::Literal(literal, location, aug) => {
                visitor.visit_literal((literal, location, aug), context, self)
            }
        }
    }
}
