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

use std::collections::HashMap;
use std::fmt::Debug;
use tree_sitter::{self, Node};

use crate::ast::Expr::BinaryExpr;
use crate::ast::Literal::{Boolean, Number};
use crate::ast::LogicOperator::{And, Or};
use crate::ast::MathOperator::{Divide, Minus, Modulo, Multiply, Plus};
use crate::ast::{Argument, BinaryInfixOperator, StringConcatOperator};
use crate::{
    ast::{self, ArgumentList, Expr, Identifier},
    grammar::GrammarNode,
};

#[derive(Debug, Clone)]
pub enum SyntaxError {
    NoSuchVisitor(String),
    EmptyContext,
    BadGrammarElement,
    BadLiteral,
    InvalidRange,
    UnexpectedExprType(String, String),
}
pub type SyntaxVisitorResult<T> = Result<T, SyntaxError>;

pub trait SyntaxVisitor<T> {
    fn visit_function_call(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_expr(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
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
    fn visit_binary_expr(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_infix_operator(
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
    fn visit_mel(
        &self,
        syntax: Node,
        context: T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
}

#[derive(Default, Clone)]
pub struct MELCompilerContext {
    pub ast: Option<Expr>,
    pub infix_operator: Option<BinaryInfixOperator>,
}

pub struct MELCompiler {
    source: String,
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

        let callee = match _driver.visit(walker.node(), self, MELCompilerContext::default())? {
            MELCompilerContext {
                ast: Some(Expr::Identifier(id)),
                infix_operator: _,
            } => Result::<Box<Identifier>, SyntaxError>::Ok(id),
            MELCompilerContext {
                ast: Some(x),
                infix_operator: _,
            } => Result::<Box<Identifier>, SyntaxError>::Err(SyntaxError::UnexpectedExprType(
                format!("{x:?}"),
                "Identifier".to_string(),
            )),
            _ => Result::<Box<Identifier>, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        let argument_list =
            match _driver.visit(walker.node(), self, MELCompilerContext::default())? {
                MELCompilerContext {
                    ast: Some(Expr::ArgumentList(args)),
                    infix_operator: _,
                } => Result::<Box<ArgumentList>, SyntaxError>::Ok(args),
                MELCompilerContext {
                    ast: Some(x),
                    infix_operator: _,
                } => Result::<Box<ArgumentList>, SyntaxError>::Err(
                    SyntaxError::UnexpectedExprType(format!("{x:?}"), "ArgumentList".to_string()),
                ),
                _ => Result::<Box<ArgumentList>, SyntaxError>::Err(SyntaxError::EmptyContext),
            }?;

        Ok(MELCompilerContext {
            ast: Some(Expr::FunctionCall(Box::new(ast::FunctionCall {
                callee: *callee,
                arguments: *argument_list,
            }))),
            infix_operator: None,
        })
    }
    fn visit_expr(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        // An expr is just a wrapper. Navigate deeper (through its named child).
        let node = syntax
            .named_child(0)
            .ok_or(SyntaxError::BadGrammarElement)?;
        driver.visit(node, self, context)
    }

    fn visit_mel(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        // A mel is just a wrapper. Navigate deeper.
        let mut walker = syntax.walk();
        walker.goto_first_child();
        driver.visit(walker.node(), self, context)
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
        };
        Ok(MELCompilerContext {
            ast: Some(Expr::Identifier(Box::new(id))),
            infix_operator: None,
        })
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
            MELCompilerContext {
                ast: Some(expr),
                infix_operator: _,
            } => Ok(MELCompilerContext {
                ast: Some(Expr::Argument(Box::new(ast::Argument { expr }))),
                infix_operator: None,
            }),
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
        let mut args: Vec<Argument> = vec![];
        for arg in syntax.named_children(&mut walker) {
            let argument = match driver.visit(arg, self, context.clone())? {
                MELCompilerContext {
                    ast: Some(Expr::Argument(arg)),
                    infix_operator: _,
                } => Result::<Box<Argument>, SyntaxError>::Ok(arg),
                MELCompilerContext {
                    ast: Some(x),
                    infix_operator: _,
                } => Result::<Box<Argument>, SyntaxError>::Err(SyntaxError::UnexpectedExprType(
                    format!("{x:?}"),
                    "Argument".to_string(),
                )),
                _ => Result::<Box<Argument>, SyntaxError>::Err(SyntaxError::EmptyContext),
            }?;
            args.push(*argument);
        }

        Ok(MELCompilerContext {
            ast: Some(Expr::ArgumentList(Box::new(ArgumentList {
                arguments: args,
            }))),
            infix_operator: None,
        })
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
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Concat(StringConcatOperator {})),
            });
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
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Logic(And)),
            });
        } else if walker.node().grammar_name() == "or" {
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Logic(Or)),
            });
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
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Math(Plus)),
            });
        } else if walker.node().grammar_name() == "minus" {
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Math(Minus)),
            });
        } else if walker.node().grammar_name() == "mul" {
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Math(Multiply)),
            });
        } else if walker.node().grammar_name() == "div" {
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Math(Divide)),
            });
        } else if walker.node().grammar_name() == "modulo" {
            return Ok(MELCompilerContext {
                ast: None,
                infix_operator: Some(BinaryInfixOperator::Math(Modulo)),
            });
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
            MELCompilerContext {
                ast: Some(left),
                infix_operator: None,
            } => Result::<Expr, SyntaxError>::Ok(left),
            _ => Result::<Expr, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        let operator = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext {
                ast: None,
                infix_operator: Some(oper),
            } => Result::<BinaryInfixOperator, SyntaxError>::Ok(oper),
            _ => Result::<BinaryInfixOperator, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        walker.goto_next_sibling();

        let right = match driver.visit(walker.node(), self, context.clone())? {
            MELCompilerContext {
                ast: Some(left),
                infix_operator: None,
            } => Result::<Expr, SyntaxError>::Ok(left),
            _ => Result::<Expr, SyntaxError>::Err(SyntaxError::EmptyContext),
        }?;

        let x = BinaryExpr(Box::new(ast::BinaryExpr {
            left,
            op: operator,
            right,
        }));
        Ok(MELCompilerContext {
            ast: Some(x),
            infix_operator: None,
        })
    }

    fn visit_infix_operator(
        &self,
        syntax: Node,
        context: MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        let mut walker = syntax.walk();
        walker.goto_first_child();
        driver.visit(walker.node(), self, context.clone())
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
            return Ok(MELCompilerContext {
                ast: Some(Expr::Literal(Box::new(Boolean(ast::BooleanLiteral::True)))),
                infix_operator: None,
            });
        } else if literal == "false" {
            return Ok(MELCompilerContext {
                ast: Some(Expr::Literal(Box::new(Boolean(ast::BooleanLiteral::False)))),
                infix_operator: None,
            });
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

        let number: usize = literal.parse().map_err(|_| SyntaxError::BadLiteral)?;
        Ok(MELCompilerContext {
            ast: Some(Expr::Literal(Box::new(Number(ast::NumberLiteral {
                literal: number,
            })))),
            infix_operator: None,
        })
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

        Ok(MELCompilerContext {
            ast: Some(Expr::Literal(Box::new(ast::Literal::String(
                ast::StringLiteral {
                    literal: literal.to_string(),
                },
            )))),
            infix_operator: None,
        })
    }
}

pub struct SyntaxVisitorDriver {}

impl SyntaxVisitorDriver {
    #[allow(dead_code)]
    pub fn visit<T, U: SyntaxVisitor<T>>(
        &self,
        node: Node,
        visitor: &U,
        context: T,
    ) -> SyntaxVisitorResult<T> {
        let hm: HashMap<String, _> = HashMap::from([
            (
                ast::FunctionCall::name(),
                U::visit_function_call as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::Expr::name(),
                U::visit_expr as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::Mel::name(),
                U::visit_mel as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::Identifier::name(),
                U::visit_identifier as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::ArgumentList::name(),
                U::visit_argument_list as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::Argument::name(),
                U::visit_argument as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::BinaryInfixOperator::name(),
                U::visit_infix_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::LogicOperator::name(),
                U::visit_logic_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::StringConcatOperator::name(),
                U::visit_string_concat_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::BinaryExpr::name(),
                U::visit_binary_expr as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::MathOperator::name(),
                U::visit_math_operator as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::Literal::name(),
                U::visit_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::BooleanLiteral::name(),
                U::visit_boolean_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::NumberLiteral::name(),
                U::visit_number_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::StringLiteral::name(),
                U::visit_string_literal as fn(&U, Node, _, &Self) -> SyntaxVisitorResult<T>,
            ),
        ]);
        match hm.get(node.grammar_name()) {
            Some(callable) => (callable)(visitor, node, context, self),
            None => Err(SyntaxError::NoSuchVisitor(node.grammar_name().into())),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompilerError {
    ParseError(String),
    SyntaxError(SyntaxError),
}
pub type CompileResult<T> = Result<T, CompilerError>;

pub fn compile(source: &str) -> CompileResult<MELCompilerContext> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_mel::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|e| CompilerError::ParseError(e.to_string()))?;
    let result = parser
        .parse(source, None)
        .ok_or(CompilerError::ParseError("Could not parse".to_string()))?;

    let vd = SyntaxVisitorDriver {};
    let sd = MELCompiler {
        source: source.into(),
    };
    let cc = MELCompilerContext::default();

    vd.visit(result.root_node(), &sd, cc)
        .map_err(CompilerError::SyntaxError)
}

#[cfg(test)]
mod tests {
    use crate::compiler::compile;

    #[test]
    fn parse_function_call() {
        let code = "testing(hello,b)";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_expr() {
        let code = "a and b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_math_expr() {
        let code = "a + b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_math_expr2() {
        let code = "a - b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_math_expr3() {
        let code = "a / b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_math_expr4() {
        let code = "a * b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_math_expr5() {
        let code = "a % b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_math_expr_with_grouping() {
        let code = "(a + b)";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_math_expr_with_grouping2() {
        let code = "(a + b) - c";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_literal() {
        let code = "true";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_number_literal() {
        let code = "5";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_string_literal() {
        let code = "\"testing\"";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }
}
