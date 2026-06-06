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

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "mel",

  rules: {
    mel: $ => choice($.list, $.expr),

    list: $ => seq("[", optional($.expr_list), "]"),

    expr_list: $ => choice($.expr, seq($.expr_list, ",", $.expr)),
    arguments: $ => choice($.expr, seq($.expr_list, ",", $.expr)),

    // Grouping expressions with parenthesis allows for precedence.
    expr: $ => choice($.grouped_expr, $.simple_expr),
    grouped_expr: $ => seq("(", $.expr, ")"),
    simple_expr: $ => choice($.assignment_expr, $.function_call_expr, $.binary_expr, $.prefix_expr, $.literal_expr),

    assignment_expr: $ => seq($.identifier, "=", $.literal),
    function_call_expr: $ => seq($.identifier, "(", optional($.arguments), ")"),

    // Assume that all binary expressions are left associative.
    binary_expr: $ => prec.left(1, seq($.expr, $.binary_infix_operator, $.expr)),
    // Assume that prefix operators are right associative (and have high precedence).
    prefix_expr: $ => prec.right(2, seq($.prefix_operator, $.expr)),
    literal_expr: $ => $.literal,

    literal: $ => choice($.string_literal, $.number_literal),
    identifier: _ => /[_a-zA-Z][_a-zA-Z\.\-\#]+/,

    number_literal: _ => /[0-9]+/,
    string_literal: _ => /"[a-z A-Z]*"/,

    // Binary infix operators.
    binary_infix_operator: $ => choice($.logic_operator, $.comparison_operator, $.math_operator, $.string_concat),
    logic_operator: $ => choice($.and, $.or),
    comparison_operator: $ => choice($.binary_eq, $.binary_lt, $.binary_lte, $.binary_gt, $.binary_gte),
    math_operator: $ => choice($.math_plus, $.math_minus, $.math_mul, $.math_div),

    prefix_operator: $ => choice($.plus, $.minus, $.bang, $.uneg),

    // Operators
    plus: _ => '+',
    minus: _ => '-',
    bang: _ => '!',
    uneg: _ => '~',

    and: _ => "and",
    or: _ => "or",

    string_concat: _ => '.',

    binary_eq: _ => "==",
    binary_lt: _ => "<",
    binary_lte: _ => "<=",
    binary_gt: _ => ">",
    binary_gte: _ => ">=",

    math_plus: _ => '+',
    math_minus: _ => '-',
    math_mul: _ => '*',
    math_div: _ => '/',

/*
<list>                ::= "[" "]" | "[" <expr_list> "]"
<expr_list>           ::= <expr> | <expr_list> "," <expr>
<expr>                ::= <identifier> "=" <expr> | <function_call> |
                          <expr> <operator> <expr> |
                          <prefix_op> <expr> | <literal>

<function_call>       ::= <function_name> "(" ")" |
                          <function_name> "(" <expr_list> ")"

<operator>            ::= <logic_operator> | <math_operator> |
                          <bit_operator> | <string_operator> |
                          <comparison_operator> | <regex_operator> |
                          <glob_operator> | <ip_operator>

<prefix_op>           ::= "+" | "-" | "!" | "~"

<literal>             ::= NUMBER | STRING | "true" | "false" | "nil"

<logic_operator>      ::= "and" | "or"

<math_operator>       ::= "+" | "-" | "*" | "/" | "%"

<bit_operator>        ::= "|" | "&" | "~" | "<<" | ">>"

<string_operator>     ::= "."

<comparison_operator> ::= "==" | "!=" | ">" | "<" | "<=" | ">="

<regex_operator>      ::= "regexmatch" | "~=" | "regexmatchi" |
                          "!regexmatch" | "!regexmatchi"

<glob_operator>       ::= "globmatch" | "*=" | "globmatchi" |
                          "%*=" | "!globmatch" | "!*=" |
                          "!globmatchi" | "!%*="

<ip_operator>         ::= "ipmatch" | "!ipmatch"

<function_name>       ::= "integer" | "real" | "string" | "boolean" |
                          "upper" | "lower" |
                          "match" | "match_replace" | "add_query" |
                          "add_query_multi" | "remove_query" |
                          "remove_query_multi" |
                          "path_element" | "path_elements"

<identifier>          ::= <alpha_char> <identifier_word> |
                          "_" <identifier_word>

<identifier_word>     ::= <identifier_char> |
                          <identifier_char> <identifier_word>

<identifier_char>     ::= <alpha_numeric> | "." | "-" | "_" | "#"

<alpha_numeric>       ::= <alpha_char> | <digit>

<alpha_char>          ::= "a-z" | "A-Z"

<digit>               ::= "0" | "1" | "2" | "3" | "4" |
                          "5" | "6" | "7" | "8" | "9"
*/
  }
});
