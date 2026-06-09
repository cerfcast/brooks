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
use tree_sitter::{self, Node, TreeCursor};

use crate::{
    ast::{self, AST, FunctionCall},
    grammar::GrammarNode,
};

#[derive(Debug, Clone)]
pub enum SyntaxError {
    NoSuchVisitor,
}
pub type SyntaxVisitorResult<T> = Result<T, SyntaxError>;

pub trait SyntaxVisitor<T> {
    fn visit_function_call(
        &self,
        syntax: Node,
        context: &mut T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_expr(
        &self,
        syntax: Node,
        context: &mut T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
    fn visit_mel(
        &self,
        syntax: Node,
        context: &mut T,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<T>;
}

#[derive(Default)]
pub struct MELCompilerContext {
    ast: Option<Box<dyn AST>>,
}

impl Debug for MELCompilerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MELCompilerContext")
            .field("ast", &self.ast)
            .finish()
    }
}

pub struct MELCompiler {}
impl SyntaxVisitor<MELCompilerContext> for MELCompiler {
    fn visit_function_call(
        &self,
        _syntax: Node,
        _context: &mut MELCompilerContext,
        _driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        Ok(MELCompilerContext {
            ast: Some(Box::new(FunctionCall {})),
        })
    }
    fn visit_expr(
        &self,
        syntax: Node,
        context: &mut MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        // An expr is just a wrapper. Navigate deeper.
        let mut walker = syntax.walk();
        walker.goto_first_child();
        driver.visit(walker, self, context)
    }
    fn visit_mel(
        &self,
        syntax: Node,
        context: &mut MELCompilerContext,
        driver: &SyntaxVisitorDriver,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        // A mel is just a wrapper. Navigate deeper.
        let mut walker = syntax.walk();
        walker.goto_first_child();
        driver.visit(walker, self, context)
    }
}

pub struct SyntaxVisitorDriver {}

impl SyntaxVisitorDriver {
    #[allow(dead_code)]
    pub fn visit<T, U: SyntaxVisitor<T>>(
        &self,
        walker: TreeCursor,
        visitor: &U,
        context: &mut T,
    ) -> SyntaxVisitorResult<T> {
        let hm: HashMap<String, _> = HashMap::from([
            (
                ast::FunctionCall::name(),
                U::visit_function_call as fn(&U, Node, &mut _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::Expr::name(),
                U::visit_expr as fn(&U, Node, &mut _, &Self) -> SyntaxVisitorResult<T>,
            ),
            (
                ast::Mel::name(),
                U::visit_mel as fn(&U, Node, &mut _, &Self) -> SyntaxVisitorResult<T>,
            ),
        ]);

        println!("Walker: {:?}", walker.node());
        match hm.get(walker.node().grammar_name()) {
            Some(callable) => (callable)(visitor, walker.node(), context, self),
            None => Err(SyntaxError::NoSuchVisitor),
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

    let walker = result.walk();
    let vd = SyntaxVisitorDriver {};
    let sd = MELCompiler {};
    let mut cc = MELCompilerContext::default();

    vd.visit(walker, &sd, &mut cc)
        .map_err(CompilerError::SyntaxError)
}

#[cfg(test)]
mod tests {
    use crate::compiler::compile;

    #[test]
    fn parse_function_call() {
        let code = "testing(a,b)";
        let compile_result = compile(code);

        assert!(compile_result.is_ok());
    }
}
