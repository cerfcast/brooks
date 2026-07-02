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

use tree_sitter;

use crate::compiler::compile::CompileResult;
use crate::compiler::compile::CompilerError;
use crate::compiler::compile::MELCompiler;
use crate::compiler::compile::MELCompilerContext;
use crate::compiler::compile::SyntaxVisitorDriver;
use crate::grammar::GrammarLocation;

pub mod compile;
#[cfg(test)]
mod test;

pub fn compile(source: &str) -> CompileResult<MELCompilerContext> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_mel::LANGUAGE;
    parser.set_language(&language.into()).map_err(|e| {
        CompilerError::SyntaxError(compile::SyntaxError::SyntaxError(
            GrammarLocation {
                start: 0,
                extent: 0,
            },
            e.to_string(),
        ))
    })?;
    let result = parser
        .parse(source, None)
        .ok_or(CompilerError::SyntaxError(
            compile::SyntaxError::SyntaxError(
                GrammarLocation {
                    start: 0,
                    extent: 0,
                },
                "Could not parse".to_string(),
            ),
        ))?;

    let vd = SyntaxVisitorDriver {};
    let sd = MELCompiler::new(source);
    let cc = MELCompilerContext::default();

    vd.visit(result.root_node(), &sd, cc)
        .map_err(CompilerError::SyntaxError)
}
