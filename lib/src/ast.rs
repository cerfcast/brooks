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

#[derive(Debug, Clone)]
#[grammar_name(mel)]
pub struct Mel {}

#[derive(Debug, Clone)]
#[grammar_name(function_call_expr)]
pub struct FunctionCall {
    pub callee: Identifier,
    pub arguments: ArgumentList,
}

#[derive(Debug, Clone)]
#[grammar_name(identifier)]
pub struct Identifier {
    pub identifier: String,
}

#[derive(Debug, Clone)]
#[grammar_name(argument)]
pub struct Argument {
    pub expr: Expr,
}

#[derive(Debug, Clone)]
#[grammar_name(argument_list)]
pub struct ArgumentList {
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Clone)]
#[grammar_name(binary_infix_operator)]
pub enum BinaryInfixOperator {
    Logic(LogicOperator),
    Comparison,
    Math(MathOperator),
    Concat,
}

#[derive(Debug, Clone)]
#[grammar_name(logic_operator)]
pub enum LogicOperator {
    And,
    Or,
}

#[derive(Debug, Clone)]
#[grammar_name(math_operator)]
pub enum MathOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Debug, Clone)]
#[grammar_name(binary_expr)]
pub struct BinaryExpr {
    pub left: Expr,
    pub op: BinaryInfixOperator,
    pub right: Expr,
}

#[derive(Debug, Clone)]
#[grammar_name(expr)]
pub enum Expr {
    FunctionCall(Box<FunctionCall>),
    BinaryExpr(Box<BinaryExpr>),
    Identifier(Box<Identifier>),
    ArgumentList(Box<ArgumentList>),
    Argument(Box<Argument>),
    Literal(Box<Literal>),
}

#[derive(Debug, Clone)]
#[grammar_name(literal)]
pub enum Literal {
    Boolean(BooleanLiteral),
}

#[derive(Debug, Clone)]
#[grammar_name(boolean_literal)]
pub enum BooleanLiteral {
    True,
    False,
}

#[derive(Debug, Clone)]
pub enum AstVisitorError {}

pub type AstVisitorResult<T> = Result<T, AstVisitorError>;

pub trait AstVisitor<T> {
    fn visit_function_call(
        &self,
        ast: FunctionCall,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T>;
    fn visit_identifier(
        &self,
        ast: Identifier,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T>;
    fn visit_argument_list(
        &self,
        ast: ArgumentList,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T>;
    fn visit_argument(
        &self,
        ast: Argument,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T>;
    fn visit_binary_expr(
        &self,
        ast: BinaryExpr,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T>;
    fn visit_literal(
        &self,
        ast: Literal,
        context: T,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<T>;
}

pub struct AstVisitorDriver {}

impl AstVisitorDriver {
    #[allow(dead_code)]
    pub fn visit<T, U: AstVisitor<T>>(
        &self,
        node: Expr,
        visitor: &U,
        context: T,
    ) -> AstVisitorResult<T> {
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
            Expr::Literal(literal) => visitor.visit_literal(*literal, context, self),
        }
    }
}
