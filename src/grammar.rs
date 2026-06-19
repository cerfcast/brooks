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

use std::fmt::Display;

/// The trait that makes it possible to add a grammar node to the compiler.
///
/// Use the `grammar_name` macro to derive it automatically.
pub trait GrammarNode {
    fn name() -> String;
}

#[derive(Debug, Clone, Default)]
pub struct GrammarLocation {
    pub start: usize,
    pub extent: usize,
}

impl Display for GrammarLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} to {}", self.start, self.start + self.extent)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Mel;
    use crate::grammar::GrammarNode;

    #[test]
    fn test_grammar_node() {
        assert_eq!(Mel::<()>::name(), "mel")
    }
}
