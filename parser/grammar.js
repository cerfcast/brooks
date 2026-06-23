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

const precedences = {
  TERNARY: 1,
  LOGIC: 2,
  COMPARISON: 3,
  CONCAT: 4,
  ADDITIVE: 5,
  MULTIPLICATIVE: 6,
  UNARY: 7,
  MEMBER_ACCESS: 8,
}
export default grammar({
  name: "mel",

  rules: {
    mel: $ => $.expr,

    expr_list: $ => choice($.expr, seq($.expr_list, ",", $.expr)),

    argument_list: $ => seq("(", optional($._arguments), ")"),
    _arguments: $ => choice($.argument, seq($._arguments, ",", $.argument)),
    //                                        ^^
    //                                      Use "hidden" production here so that
    //                                      grammar nodes do not get annoyingly,
    //                                      deeply nested. 

    // An argument is just an alias for an expression. The special name
    // will make parse trees more self documenting.
    argument: $ => $._hidden_expr,

    expr: $ => $._hidden_expr,

    // Grouping expressions with parenthesis allows for precedence.
    _hidden_expr: $ => choice($._grouped_expr, $._simple_expr),

    _grouped_expr: $ => seq("(", $._hidden_expr, ")"),
    _simple_expr: $ => choice($.assignment_expr, $.function_call_expr, $.binary_expr, $.prefix_expr, $.ternary_expr, $._literal_expr, $._variable),

    assignment_expr: $ => seq($._variable, "=", $.literal),
    function_call_expr: $ => seq($._variable, $.argument_list),

    binary_expr: $ => choice(
    ...[
      [$.and, precedences.LOGIC],
      [$.or, precedences.LOGIC],

      [$.lt, precedences.COMPARISON],
      [$.lte, precedences.COMPARISON],
      [$.gt, precedences.COMPARISON],
      [$.gte, precedences.COMPARISON],
      [$.eq, precedences.COMPARISON],
      [$.ne, precedences.COMPARISON],
      [$.regex_eq, precedences.COMPARISON],

      [$.string_concat, precedences.CONCAT],

      [$.plus, precedences.ADDITIVE],
      [$.minus, precedences.ADDITIVE],

      [$.mul, precedences.MULTIPLICATIVE],
      [$.div, precedences.MULTIPLICATIVE],
      [$.modulo, precedences.MULTIPLICATIVE],
    ].map(([op, precedence]) => 
      prec.left(precedence, seq($.expr, op, $.expr))
    )),

    prefix_expr: $ => choice(
      ...[
        [$.bang, precedences.UNARY]
      ].map(([op, precedence]) => 
        prec.right(precedence, seq(op, $.expr))
      )
    ),

    _variable: $=> choice($.identifier, $.member_access_expr), 
    member_access_expr: $=> prec.left(precedences.MEMBER_ACCESS, seq($._variable, $.member_access, $.identifier)),
    //                                                                             ^^
    //                                                                             Do not use "hidden" production here -- nested AST
    //                                                                             will make type inference easier.

    ternary_expr: $ => choice(prec.right(precedences.TERNARY, seq($.expr, $.ternary_question, $.expr, $.ternary_colon, $.expr))),

    _literal_expr: $ => $.literal,

    literal: $ => choice($.string_literal, $.number_literal, $.boolean_literal, $.regex_literal),
    identifier: _ => /[_a-zA-Z][_a-zA-Z]*/,

    number_literal: _ => /[0-9]+/,
    string_literal: _ => /"[a-z A-Z]*"/,
    boolean_literal: _ => choice("true", "false"),
    regex_literal: _ => /"\/[()*.+a-z$\^#\\:|\[\]\?A-Z]*\/i?"/,

    // Operators
    plus: _ => '+',
    minus: _ => '-',
    bang: _ => '!',
    uneg: _ => '~',

    ternary_question: _ => '?',
    ternary_colon: _ => ':',

    and: _ => "and",
    or: _ => "or",

    string_concat: _ => '.',

    member_access: _ => '^',

    eq: _ => "==",
    ne: _ => "!=",
    regex_eq: _ => "~=",
    lt: _ => "<",
    lte: _ => "<=",
    gt: _ => ">",
    gte: _ => ">=",

    mul: _ => '*',
    div: _ => '/',
    modulo: _ => '%',
  }
});
