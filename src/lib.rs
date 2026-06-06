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

#[cfg(test)]
mod tests {
    #[test]
    fn parse_expression_list() {
        let code = "[ testing = 5 ]";

        let mut parser = tree_sitter::Parser::new();
        let language = tree_sitter_mel::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("Error loading Mel parser");
        let _ = parser
            .parse(code, None)
            .expect("Parsing simple expression list should succeed");
    }
}
