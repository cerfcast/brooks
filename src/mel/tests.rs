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
mod builtin_function_types_tests {
    use std::assert_matches;

    use crate::mel::tvs::{
        Args, BuiltinFunctionType, IntegerBuiltin, ParamsTypeCheckerError, Type,
    };

    #[test]
    fn test_typecheck_builtin_function_integer_return_type() {
        let b = IntegerBuiltin {};

        let return_type = b.return_type_calculator()();
        assert_eq!(return_type, Type::Integer);
    }

    #[test]
    fn test_typecheck_builtin_function_integer_parameter_count_ok() {
        let b = IntegerBuiltin {};

        let ptc = b.params_type_checker()();

        let arg_types = Args {
            args: vec![Type::Integer],
        };
        assert_matches!(ptc.check(arg_types), Ok(_));
    }

    fn test_typecheck_builtin_function_integer_miscount() {
        let b = IntegerBuiltin {};
        let ptc = b.params_type_checker()();

        let arg_types = Args {
            args: vec![Type::Integer, Type::Integer],
        };
        assert_matches!(
            ptc.check(arg_types),
            Err(ParamsTypeCheckerError::Miscount(1, 2))
        );
    }
}
