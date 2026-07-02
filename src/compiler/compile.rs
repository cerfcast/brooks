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

use regex::RegexBuilder;
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::IpAddr;
use std::sync::Arc;
use tree_sitter::{self, Node};

use crate::ast::Expr::BinaryExpr;
use crate::ast::Expr::MemberAccess;
use crate::ast::Expr::TernaryExpr;
use crate::ast::Literal::{Boolean, Number};
use crate::ast::LogicOperator::{And, Or};
use crate::ast::MathOperator::{Divide, Minus, Modulo, Multiply, Plus};
use crate::ast::{Argument, BinaryInfixOperator, StringConcatOperator};
use crate::grammar::GrammarLocation;
use crate::utils;
use crate::{
    ast::{self, ArgumentList, Expr, Identifier},
    grammar::GrammarNode,
};

#[derive(Debug, Clone)]
pub enum SyntaxError {
    NoSuchVisitor(String),
    EmptyContext,
    BadGrammarElement,
    BadLiteral(String),
    InvalidRange,
    UnexpectedExprType(String, String),
    SyntaxError(GrammarLocation, String),
}
pub type SyntaxVisitorResult<T> = Result<T, SyntaxError>;

pub trait SyntaxVisitor<T>: Sized {
    fn visit_function_call(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_error(&self, syntax: Node, context: T, driver: &SyntaxVisitorDriver) -> SyntaxError;
    fn visit_expr(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T> {
        let node = syntax
            .named_child(0)
            .ok_or(SyntaxError::BadGrammarElement)?;
        driver.visit(node, self, context)
    }
    fn visit_identifier(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_argument_list(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_argument(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_string_concat_operator(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_comparison_operator(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_logic_operator(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_math_operator(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_member_access_expr(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_member_access_oper(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_binary_expr(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_ternary_operator(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_ternary_expr(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_literal(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_boolean_literal(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_number_literal(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_string_literal(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_regex_literal(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_ipaddress_literal(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_mel(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
}

/// A context for compilation used when visiting the parse tree.
#[derive(Default, Debug, Clone)]
pub enum MELCompilerContext {
    Expr(ast::Expr<()>),
    BinaryOperator(ast::BinaryInfixOperator),
    TernaryOperator(ast::TernaryOperator),
    #[default]
    Empty,
}

/// Expect that the [`MELCompilerContext`] has an Expr.
#[macro_export]
macro_rules! expect_expr {
    ( $t:ident, $x:expr ) => {{
        match $x {
            $t::Expr(e) => Some(e),
            _ => None,
        }
    }};
}

pub struct MELCompiler {
    source: String,
}

impl MELCompiler {
    pub fn new(source: &str) -> Self {
        MELCompiler {
            source: source.to_string(),
        }
    }

    pub fn extract_error(&self, en: &Node) -> SyntaxError {
        let error_tok = en
            .utf8_text(self.source.as_bytes())
            .or::<String>(Ok(""))
            .expect("Could not get error token");

        let repr = en
            .to_string()
            .trim_start_matches("(")
            .trim_end_matches(")")
            .to_string();

        let msg = if repr.starts_with("UNEXPECTED") {
            format!("Unexpected token {error_tok}")
        } else if repr.starts_with("MISSING") {
            let msg = repr.trim_start_matches("MISSING ");
            format!("Missing {msg}")
        } else {
            repr
        };

        SyntaxError::SyntaxError(Into::<GrammarLocation>::into(*en), msg)
    }
}

impl SyntaxVisitor<MELCompilerContext> for MELCompiler {
    fn visit_function_call(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();

        let callee = match _driver.visit(walker.node(), self, _context.clone())? {
            MELCompilerContext::Expr(callee) => Result::<Expr<()>, SyntaxError>::Ok(callee),
            _ => Result::<Expr<()>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        let argument_list = match _driver.visit(walker.node(), self, _context.clone())? {
            MELCompilerContext::Expr(ast::Expr::ArgumentList(args)) => {
                Result::<Arc<ArgumentList<()>>, SyntaxError>::Ok(args)
            }
            _ => Result::<Arc<ArgumentList<()>>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        Ok(MELCompilerContext::Expr(Expr::FunctionCall(Arc::new(
            ast::FunctionCall::<()> {
                callee: callee.clone(),
                arguments: (*argument_list).clone(),
                location: syntax.into(),
                aug: (),
            },
        ))))
    }

    fn visit_mel(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        // A mel is just a wrapper. Navigate deeper.
        let mut context = context;
        for child in syntax.children(&mut syntax.walk()) {
            context = driver.visit(child, self, context)?;
        }

        Ok(context)
    }

    fn visit_identifier(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let identifier = syntax
            .utf8_text(self.source.as_bytes())
            .map_err(|_| SyntaxError::InvalidRange)?;
        let id = Identifier {
            identifier: identifier.to_string(),
            location: syntax.into(),
            aug: (),
        };
        Ok(MELCompilerContext::Expr(Expr::Identifier(Arc::new(id))))
    }

    fn visit_argument(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        // An argument is just a wrapper around an expr!
        let mut walker = syntax.walk();
        walker.goto_first_child();

        match _driver.visit(walker.node(), self, _context.clone())? {
            MELCompilerContext::Expr(expr) => Ok(MELCompilerContext::Expr(Expr::Argument(
                Arc::new(ast::Argument {
                    expr,
                    location: syntax.into(),
                    aug: (),
                }),
            ))),
            _ => Err(SyntaxError::EmptyContext),
        }
    }

    fn visit_argument_list(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        let mut args: Vec<Argument<()>> = vec![];
        for arg in syntax.named_children(&mut walker) {
            let argument = match driver.visit(arg, self, context.clone())? {
                MELCompilerContext::Expr(Expr::Argument(arg)) => {
                    Result::<Arc<Argument<()>>, SyntaxError>::Ok(arg)
                }
                _ => Result::<Arc<Argument<()>>, SyntaxError>::Err(SyntaxError::EmptyContext),
            }?;
            args.push((*argument).clone());
        }

        Ok(MELCompilerContext::Expr(Expr::ArgumentList(Arc::new(
            ArgumentList {
                arguments: args,
                location: syntax.into(),
                aug: (),
            },
        ))))
    }

    fn visit_string_concat_operator(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();

        if walker.node().grammar_name() == "string_concat" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Concat(StringConcatOperator {}),
            ));
        }
        Err(SyntaxError::BadGrammarElement)
    }

    fn visit_logic_operator(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();

        if walker.node().grammar_name() == "and" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Logic(And),
            ));
        } else if walker.node().grammar_name() == "or" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Logic(Or),
            ));
        }
        Err(SyntaxError::BadGrammarElement)
    }

    fn visit_math_operator(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();

        if walker.node().grammar_name() == "plus" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Math(Plus),
            ));
        } else if walker.node().grammar_name() == "minus" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Math(Minus),
            ));
        } else if walker.node().grammar_name() == "mul" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Math(Multiply),
            ));
        } else if walker.node().grammar_name() == "div" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Math(Divide),
            ));
        } else if walker.node().grammar_name() == "modulo" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Math(Modulo),
            ));
        }
        Err(SyntaxError::BadGrammarElement)
    }

    fn visit_binary_expr(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();

        walker.goto_first_child();

        let left = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::Expr(left) => Result::<Expr<()>, SyntaxError>::Ok(left),
            _ => Result::<Expr<()>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        let operator = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::BinaryOperator(oper) => {
                Result::<BinaryInfixOperator, SyntaxError>::Ok(oper)
            }
            _ => Result::<BinaryInfixOperator, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        let right = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::Expr(right) => Result::<Expr<()>, SyntaxError>::Ok(right),
            _ => Result::<Expr<()>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        Ok(MELCompilerContext::Expr(BinaryExpr(Arc::new(
            ast::BinaryExpr {
                left,
                op: operator,
                right,
                location: syntax.into(),
                aug: (),
            },
        ))))
    }

    fn visit_ternary_operator(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();
        if walker.node().grammar_name() == "ternary_question" {
            return Ok(MELCompilerContext::TernaryOperator(
                ast::TernaryOperator::Question,
            ));
        } else if walker.node().grammar_name() == "ternary_colon" {
            return Ok(MELCompilerContext::TernaryOperator(
                ast::TernaryOperator::Colon,
            ));
        }
        Err(SyntaxError::BadGrammarElement)
    }

    fn visit_ternary_expr(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();

        walker.goto_first_child();

        let condition = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::Expr(condition) => Result::<Expr<()>, SyntaxError>::Ok(condition),
            _ => Result::<Expr<()>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::TernaryOperator(ast::TernaryOperator::Question) => {
                Result::<(), SyntaxError>::Ok(())
            }
            _ => Result::<(), SyntaxError>::Err(SyntaxError::BadGrammarElement),
        }?;

        walker.goto_next_sibling();

        let yes = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::Expr(yes) => Result::<Expr<()>, SyntaxError>::Ok(yes),
            _ => Result::<Expr<()>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::TernaryOperator(ast::TernaryOperator::Colon) => {
                Result::<(), SyntaxError>::Ok(())
            }
            _ => Result::<(), SyntaxError>::Err(SyntaxError::BadGrammarElement),
        }?;

        walker.goto_next_sibling();

        let no = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::Expr(no) => Result::<Expr<()>, SyntaxError>::Ok(no),
            _ => Result::<Expr<()>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        Ok(MELCompilerContext::Expr(TernaryExpr(Arc::new(
            ast::TernaryExpr {
                condition,
                yes,
                no,
                location: syntax.into(),
                aug: (),
            },
        ))))
    }

    fn visit_literal(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();
        driver.visit(walker.node(), self, context)
    }

    fn visit_boolean_literal(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let literal = syntax
            .utf8_text(self.source.as_bytes())
            .map_err(|_| SyntaxError::InvalidRange)?;
        if literal == "true" {
            return Ok(MELCompilerContext::Expr(Expr::Literal(
                Boolean(ast::BooleanLiteral::True),
                syntax.into(),
                (),
            )));
        } else if literal == "false" {
            return Ok(MELCompilerContext::Expr(Expr::Literal(
                Boolean(ast::BooleanLiteral::False),
                syntax.into(),
                (),
            )));
        }
        Err(SyntaxError::BadGrammarElement)
    }
    fn visit_number_literal(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let literal = syntax
            .utf8_text(self.source.as_bytes())
            .map_err(|_| SyntaxError::InvalidRange)?;

        let number: usize = literal
            .parse::<usize>()
            .map_err(|e| SyntaxError::BadLiteral(e.to_string()))?;
        Ok(MELCompilerContext::Expr(Expr::Literal(
            Number(ast::NumberLiteral { literal: number }),
            syntax.into(),
            (),
        )))
    }

    fn visit_string_literal(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let literal = syntax
            .utf8_text(self.source.as_bytes())
            .map_err(|_| SyntaxError::InvalidRange)?;

        Ok(MELCompilerContext::Expr(Expr::Literal(
            ast::Literal::String(ast::StringLiteral {
                literal: utils::strip_quotes(literal),
            }),
            syntax.into(),
            (),
        )))
    }

    fn visit_comparison_operator(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();

        if walker.node().grammar_name() == "eq" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::Eq),
            ));
        } else if walker.node().grammar_name() == "lt" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::Lt),
            ));
        } else if walker.node().grammar_name() == "lte" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::Lte),
            ));
        } else if walker.node().grammar_name() == "gt" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::Gt),
            ));
        } else if walker.node().grammar_name() == "gte" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::Gte),
            ));
        } else if walker.node().grammar_name() == "ne" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::Ne),
            ));
        } else if walker.node().grammar_name() == "regex_eq" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::Re),
            ));
        } else if walker.node().grammar_name() == "ipmatch" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::Comparison(ast::ComparisonOperator::IP),
            ));
        }

        Err(SyntaxError::BadGrammarElement)
    }

    fn visit_member_access_expr(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();

        walker.goto_first_child();

        let base = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::Expr(base) => Result::<Expr<()>, SyntaxError>::Ok(base),
            _ => Result::<Expr<()>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::BinaryOperator(BinaryInfixOperator::MemberAccess(_)) => {
                Result::<(), SyntaxError>::Ok(())
            }
            _ => Result::<(), SyntaxError>::Err(SyntaxError::BadGrammarElement),
        }?;

        walker.goto_next_sibling();

        let member = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext::Expr(ast::Expr::Identifier(id)) => {
                Result::<Arc<Identifier<()>>, SyntaxError>::Ok(id)
            }
            _ => Result::<Arc<Identifier<()>>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        Ok(MELCompilerContext::Expr(MemberAccess(Arc::new(
            ast::MemberAccessExpression {
                base,
                member: (*member).clone(),
                oper: ast::MemberAccessOperator::MemberAccess,
                location: syntax.into(),
                aug: (),
            },
        ))))
    }

    fn visit_member_access_oper(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();

        if walker.node().grammar_name() == "member_access" {
            return Ok(MELCompilerContext::BinaryOperator(
                BinaryInfixOperator::MemberAccess(ast::MemberAccessOperator::MemberAccess),
            ));
        }
        Err(SyntaxError::BadGrammarElement)
    }

    fn visit_regex_literal(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let literal = syntax
            .utf8_text(self.source.as_bytes())
            .map_err(|_| SyntaxError::InvalidRange)?;

        let regex = match RegexBuilder::new(&utils::strip_quotes(literal)).build() {
            Ok(regex) => regex,
            Err(e) => return Err(SyntaxError::BadLiteral(e.to_string())),
        };

        Ok(MELCompilerContext::Expr(Expr::Literal(
            ast::Literal::Regex(ast::RegexLiteral { literal: regex }),
            syntax.into(),
            (),
        )))
    }

    fn visit_ipaddress_literal(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let literal = syntax
            .utf8_text(self.source.as_bytes())
            .map_err(|_| SyntaxError::InvalidRange)?;

        let ip_addr: IpAddr = literal
            .parse::<IpAddr>()
            .map_err(|e| SyntaxError::BadLiteral(e.to_string()))?;

        Ok(MELCompilerContext::Expr(Expr::Literal(
            ast::Literal::IPAddress(ast::IPAddressLiteral { literal: ip_addr }),
            syntax.into(),
            (),
        )))
    }

    fn visit_error(
        &self,
        syntax: Node,
        _context: MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxError {
        // if the node is an error, then the error is really in the child.
        if syntax.is_error()
            && let Some(child) = syntax.child(0)
        {
            self.extract_error(&child)
        // if the node is missing, then the error is right here.
        } else if syntax.is_missing() {
            self.extract_error(&syntax)
        } else {
            SyntaxError::SyntaxError(
                Into::<GrammarLocation>::into(syntax),
                "Unknown syntax error (couldn't get token)".to_string(),
            )
        }
    }
}

pub struct SyntaxVisitorDriver {}

impl SyntaxVisitorDriver {
    pub fn visit<T, U: SyntaxVisitor<T>>(
        &self,
        node: Node,
        visitor: &U,
        context: T,
    ) -> SyntaxVisitorResult<T> {
        type Visitor<T, U> = fn(&U, Node, T, &SyntaxVisitorDriver) -> SyntaxVisitorResult<T>;

        let mut hm: HashMap<String, Visitor<T, U>> = HashMap::new();

        // Register each of the appropriate visitor functions with the grammar nodes (as
        // defined by the grammar_node macro).
        ast::FunctionCall::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_function_call as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::Expr::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_expr as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::Mel::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_mel as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::Identifier::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_identifier as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::ArgumentList::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_argument_list as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::Argument::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_argument as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::LogicOperator::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_logic_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::StringConcatOperator::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_string_concat_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::ComparisonOperator::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_comparison_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::BinaryExpr::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_binary_expr as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::MemberAccessExpression::<()>::name()
            .iter()
            .for_each(|name| {
                hm.insert(
                    name.clone(),
                    U::visit_member_access_expr as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
                );
            });
        ast::MemberAccessOperator::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_member_access_oper as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::TernaryExpr::<()>::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_ternary_expr as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::MathOperator::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_math_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::TernaryOperator::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_ternary_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::Literal::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::BooleanLiteral::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_boolean_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::NumberLiteral::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_number_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::StringLiteral::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_string_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::RegexLiteral::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_regex_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });
        ast::IPAddressLiteral::name().iter().for_each(|name| {
            hm.insert(
                name.clone(),
                U::visit_ipaddress_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            );
        });

        // Special case for errors.
        if node.is_error() || node.is_missing() || node.is_extra() {
            Err(visitor.visit_error(node, context, self))
        } else {
            match hm.get(node.grammar_name()) {
                Some(callable) => (callable)(visitor, node, context, self),
                None => Err(SyntaxError::NoSuchVisitor(node.grammar_name().into())),
            }
        }
    }
}

impl From<Node<'_>> for GrammarLocation {
    fn from(value: Node) -> Self {
        GrammarLocation {
            start: value.start_byte(),
            extent: value.end_byte() - value.start_byte(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompilerError {
    SyntaxError(SyntaxError),
}
pub type CompileResult<T> = Result<T, CompilerError>;
