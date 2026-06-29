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

pub mod builtins;
pub mod interpret;
pub mod tests;

use crate::{
    analysis::Analyzed,
    ast::{AstVisitorDriver, Expr},
    interpreter::interpret::{
        MelInterp, MelInterpAssertion::SuccessWithoutValue, MelInterpContext, MelInterpError,
        MelInterpLocatableError, MelInterpResult, TypedValue,
    },
    scope::Scopes,
};

#[allow(clippy::result_large_err)]
pub fn interpret(expr: &Expr<Analyzed>, scopes: Scopes<TypedValue>) -> MelInterpResult {
    let driver = AstVisitorDriver {};
    let visitor = MelInterp {};
    let mut context = MelInterpContext::default();

    context = context.update_scopes(scopes);

    match driver.visit(expr, &visitor, context)?.val {
        Some(v) => Ok(v),
        None => Err(MelInterpLocatableError {
            error: MelInterpError::Assertion(SuccessWithoutValue(
                "main".to_string(),
                "interpret".to_string(),
            )),
            location: expr.location(),
        }),
    }
}
