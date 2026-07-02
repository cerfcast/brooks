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
    use crate::compiler::compile;
    use crate::compiler::compile::CompilerError;
    use std::assert_matches;

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
    fn parse_binary_comparison() {
        let code = "a == b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_comparison2() {
        let code = "a != b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_comparison3() {
        let code = "a <= b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_binary_comparison4() {
        let code = "a < b";
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

    #[test]
    fn parse_ternary_expr() {
        let code = "true ? a : b";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_ternary_expr_associativity_check() {
        let code = "true ? a : false ? \"b\" : 10";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_regex_literal() {
        let code = "/[\\W]/i";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_regex_literal_empty_character_set() {
        let code = "/[]/";
        let compile_result = compile(code).expect_err("Could compile code with syntax error");
        assert_matches!(compile_result, CompilerError::BadLiteral(_))
    }

    #[test]
    fn parse_ip_literal() {
        let code = "192.168.0.1";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_ipv6_literal() {
        let code = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_ipmatch_binary_expr_ipv6_literal() {
        let code = "a ipmatch 2001:0db8:85a3:0000:0000:8a2e:0370:7334";
        let compile_result = compile(code);
        assert!(compile_result.is_ok());
    }

    #[test]
    fn parse_simple_error_unexpected() {
        let code = "/testing/j";
        let compile_result = compile(code).expect_err("Could compile code with syntax error");

        assert_matches!(compile_result,
            CompilerError::SyntaxError(_,msg)
            if msg == "Unexpected token j");
    }

    #[test]
    fn parse_simple_error_missing() {
        let code = "/testing";
        let compile_result = compile(code).expect_err("Could compile code with syntax error");

        assert_matches!(compile_result,
            CompilerError::SyntaxError(_,msg)
            if msg == "Missing identifier");
    }
}
