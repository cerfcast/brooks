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

use tree_sitter::{self, Node, TreeCursor};

use crate::ast::{FunctionCall, MELAST};

#[derive(Debug, Clone)]
pub struct SyntaxError {}
pub type SyntaxVisitorResult<T> = Result<T, SyntaxError>;

pub trait SyntaxVisitor<T> {
    fn visit_function_call(&self, syntax: Node, context: T) -> SyntaxVisitorResult<T>;
}

pub struct MELCompiler {}

pub struct MELCompilerContext {}

impl SyntaxVisitor<MELCompilerContext> for MELCompiler {
    fn visit_function_call(
        &self,
        _syntax: Node,
        context: MELCompilerContext,
    ) -> SyntaxVisitorResult<MELCompilerContext> {
        Ok(context)
    }
}

pub struct SyntaxVisitorDriver {}

impl SyntaxVisitorDriver {
    #[allow(dead_code)]
    pub fn visit<T>(
        &self,
        mut walker: TreeCursor,
        visitor: impl SyntaxVisitor<T>,
        context: T,
    ) -> SyntaxVisitorResult<T> {
        println!("Walker: {:?}", walker.node());
        let grammar_name = walker.node().grammar_name();
        if grammar_name == "function_call_expr" {
            visitor.visit_function_call(walker.node(), context)
        } else if grammar_name == "mel" || grammar_name == "expr" || grammar_name == "simple_expr" {
            walker.goto_first_child();
            self.visit(walker, visitor, context)
        } else {
            SyntaxVisitorResult::<T>::Err(SyntaxError {})
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompilerError {
    ParseError(String),
    SyntaxError(SyntaxError),
}
pub type CompileResult<T> = Result<T, CompilerError>;

pub fn compile(source: &str) -> CompileResult<impl MELAST> {
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
    let cd = MELCompilerContext {};

    vd.visit(walker, sd, cd)
        .map_err(CompilerError::SyntaxError)
        .map(|_| FunctionCall {})
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
