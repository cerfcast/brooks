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

//! MEL Types and Values

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use brooks_macros::builtin_function;
use regex::Regex;

use crate::{
    environment::scope::Scope,
    mel::{MEL_USER_SET_VARIABLE_STRUCT_NAME, interpreter::interpret::BuiltinFunction},
    prr::Prr,
};

/// Types

#[derive(Debug, Clone, Default)]
pub struct Struct {
    pub name: String,
    fields: HashMap<String, Type>,
    wild: Vec<(Regex, Type)>,
}

impl PartialEq for Struct {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Struct {
    pub fn new(name: &str) -> Struct {
        Struct {
            name: name.to_string(),
            fields: HashMap::new(),
            wild: vec![],
        }
    }

    pub fn insert_field(&mut self, name: &str, tipe: Type) {
        self.fields.insert(name.to_string(), tipe);
    }

    pub fn insert_wild_field(&mut self, matcher: &Regex, tipe: Type) {
        self.wild.push((matcher.clone(), tipe));
    }

    pub fn get_field(&self, name: &str) -> Option<Type> {
        if let Some(exact_field) = self.fields.get(name) {
            Some(exact_field.clone())
        } else {
            self.wild.iter().find_map(|(regex, tpe)| {
                if regex.is_match(name) {
                    Some(tpe.clone())
                } else {
                    None
                }
            })
        }
    }

    pub fn fields(&self) -> impl Iterator<Item = (Option<&String>, Option<&Regex>)> {
        self.fields
            .keys()
            .map(|f| (Some(f), None))
            .chain(self.wild.iter().map(|f| (None, Some(&f.0))))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArgumentTypeList {
    pub args: Vec<Type>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParameterTypesList {
    pub params: Vec<Type>,
}

pub type ReturnTypeCalculator = fn() -> Type;

#[derive(Debug, Clone)]
pub enum ParamsTypeCheckerError {
    Miscount(usize, usize),
    Mismatch(Type, Type),
}

#[allow(clippy::result_large_err)]
pub trait ParamsTypeChecker: Display {
    fn check(&self, args: ArgumentTypeList) -> Result<(), ParamsTypeCheckerError>;
}
pub type ParamsTypeCheckerGenerator = fn() -> Box<dyn ParamsTypeChecker>;

#[derive(Default, Debug, Clone)]
pub enum Type {
    Boolean,
    Integer,
    Unsigned,
    Real,
    String,
    Regex,
    IPAddress,
    Params(ParameterTypesList),
    Function(String, ReturnTypeCalculator, ParamsTypeCheckerGenerator),
    Struct(Struct),
    #[default]
    Nil,
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Params(left), Self::Params(right)) => left == right,
            (Self::Function(name, _, _), Self::Function(other_name, _, _)) => name == other_name,
            (Self::Struct(left), Self::Struct(right)) => left == right,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Display for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {}, Fields: {}",
            self.name,
            self.fields.keys().cloned().collect::<Vec<_>>().join(","),
        )
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Unsigned => write!(f, "Unsigned"),
            Type::Boolean => write!(f, "Bool"),
            Type::Integer => write!(f, "Integer"),
            Type::Real => write!(f, "Real"),
            Type::String => write!(f, "String"),
            Type::Regex => write!(f, "Regex"),
            Type::IPAddress => write!(f, "IPAddress"),
            Type::Params(items) => write!(
                f,
                "Parameters: {}",
                items
                    .params
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Type::Function(name, rtg, ptcg) => {
                let rt = rtg();
                let ptc = ptcg();
                write!(f, "Name: {name} Return Type: {rt}, Parameter Types: {ptc}")
            }
            Type::Struct(s) => write!(f, "Struct: {s}"),
            Type::Nil => write!(f, "Nil"),
        }
    }
}

impl Struct {
    pub fn type_for_field(&self, field_name: &str) -> Option<Type> {
        self.fields.get(field_name).cloned()
    }
}

pub struct SimpleParamTypeChecker {
    pub p: ParameterTypesList,
}

impl ParamsTypeChecker for SimpleParamTypeChecker {
    fn check(&self, a: ArgumentTypeList) -> Result<(), ParamsTypeCheckerError> {
        if self.p.params.len() != a.args.len() {
            return Err(ParamsTypeCheckerError::Miscount(
                self.p.params.len(),
                a.args.len(),
            ));
        }

        for (p, a) in self.p.params.iter().zip(&a.args) {
            if *p != *a {
                return Err(ParamsTypeCheckerError::Mismatch(p.clone(), a.clone()));
            }
        }

        Ok(())
    }
}

impl Display for SimpleParamTypeChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.p
                .params
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

pub struct TypelessParamTypeChecker {
    pub count: usize,
}

impl ParamsTypeChecker for TypelessParamTypeChecker {
    fn check(&self, a: ArgumentTypeList) -> Result<(), ParamsTypeCheckerError> {
        if self.count != a.args.len() {
            return Err(ParamsTypeCheckerError::Miscount(self.count, a.args.len()));
        }

        Ok(())
    }
}

impl Display for TypelessParamTypeChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ["Any"].repeat(self.count).join(","))
    }
}

pub trait BuiltinFunctionType: Debug {
    fn name(&self) -> String;
    fn params_type_checker(&self) -> ParamsTypeCheckerGenerator;
    fn return_type_calculator(&self) -> ReturnTypeCalculator;
}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::Integer)]
pub struct Path_ElementBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::Integer, Type::Integer)]
pub struct Path_ElementsBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::String)]
pub struct MatchBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::String, Type::String)]
pub struct Match_ReplaceBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::String, Type::String)]
pub struct Add_QueryBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::String)]
pub struct Add_Query_MultiBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::String)]
pub struct Remove_QueryBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::String)]
pub struct Remove_Query_MultiBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::String)]
pub struct Keep_Query_MultiBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String)]
pub struct LowerBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String)]
pub struct UpperBuiltin {}

#[derive(Clone, Default, Debug)]
pub struct BooleanBuiltin {}
impl BuiltinFunction for BooleanBuiltin {}
impl BuiltinFunctionType for BooleanBuiltin {
    fn name(&self) -> String {
        "boolean".to_string()
    }

    fn params_type_checker(&self) -> ParamsTypeCheckerGenerator {
        || Box::new(TypelessParamTypeChecker { count: 1 })
    }

    fn return_type_calculator(&self) -> ReturnTypeCalculator {
        || Type::Boolean
    }
}

#[derive(Clone, Default, Debug)]
pub struct IntegerBuiltin {}
impl BuiltinFunction for IntegerBuiltin {}
impl BuiltinFunctionType for IntegerBuiltin {
    fn name(&self) -> String {
        "integer".to_string()
    }

    fn params_type_checker(&self) -> ParamsTypeCheckerGenerator {
        || Box::new(TypelessParamTypeChecker { count: 1 })
    }

    fn return_type_calculator(&self) -> ReturnTypeCalculator {
        || Type::Integer
    }
}

#[derive(Clone, Default, Debug)]
pub struct UnsignedBuiltin {}
impl BuiltinFunction for UnsignedBuiltin {}
impl BuiltinFunctionType for UnsignedBuiltin {
    fn name(&self) -> String {
        "unsigned".to_string()
    }

    fn params_type_checker(&self) -> ParamsTypeCheckerGenerator {
        || Box::new(TypelessParamTypeChecker { count: 1 })
    }

    fn return_type_calculator(&self) -> ReturnTypeCalculator {
        || Type::Unsigned
    }
}

#[derive(Clone, Default, Debug)]
pub struct RealBuiltin {}
impl BuiltinFunction for RealBuiltin {}
impl BuiltinFunctionType for RealBuiltin {
    fn name(&self) -> String {
        "real".to_string()
    }

    fn params_type_checker(&self) -> ParamsTypeCheckerGenerator {
        || Box::new(TypelessParamTypeChecker { count: 1 })
    }

    fn return_type_calculator(&self) -> ReturnTypeCalculator {
        || Type::Real
    }
}

#[derive(Clone, Default, Debug)]
pub struct StringBuiltin {}
impl BuiltinFunction for StringBuiltin {}
impl BuiltinFunctionType for StringBuiltin {
    fn name(&self) -> String {
        "string".to_string()
    }

    fn params_type_checker(&self) -> ParamsTypeCheckerGenerator {
        || Box::new(TypelessParamTypeChecker { count: 1 })
    }

    fn return_type_calculator(&self) -> ReturnTypeCalculator {
        || Type::String
    }
}

pub fn mel_empty_header_type(wild: bool) -> Struct {
    let mut ht = Struct::new("h");
    if wild {
        ht.insert_wild_field(
            &Regex::new(".*").expect("Could not compile wildcard regex for header type"),
            Type::String,
        );
    }

    ht
}

pub fn mel_header_type_from_req<A>(value: &http::Request<A>) -> Struct {
    // Make the header type.
    let mut ht = Struct::new("h");
    value.headers().iter().for_each(|header| {
        ht.insert_field(
            &header.0.to_string().replace("-", "_").to_lowercase(),
            Type::String,
        );
    });
    ht
}

pub fn mel_uri_type() -> Struct {
    // Make the URI type.
    let mut urit = Struct::new("uri");
    urit.insert_field("path", Type::String);
    urit.insert_field("query", Type::String);

    urit
}

pub fn mel_req_type(header_type: Struct, uri_type: Struct) -> Struct {
    // Make the req type.
    let mut reqs = Struct::new("req");
    reqs.insert_field("h", Type::Struct(header_type));
    reqs.insert_field("uri", Type::Struct(uri_type));
    reqs.insert_field("method", Type::String);
    reqs.insert_field("scheme", Type::String);
    reqs.insert_field("clientip", Type::IPAddress);
    reqs.insert_field("clientport", Type::Integer);

    reqs
}

pub fn mel_empty_var_type() -> Struct {
    Struct::new(MEL_USER_SET_VARIABLE_STRUCT_NAME)
}

pub(crate) fn mel_header_type_from_processable_rr<A>(value: &dyn Prr<A>) -> Struct {
    // Make the header type.
    let mut ht = Struct::new("h");
    value.headers().iter().for_each(|header| {
        ht.insert_field(
            &header.0.to_string().replace("-", "_").to_lowercase(),
            Type::String,
        );
    });
    ht
}

/// Create a scope that contains the types of the MEL core variables.
pub fn minimal_core_variable_types() -> Scope<Type> {
    let mut scope = Scope::<Type>::default();
    let ht = mel_empty_header_type(true);
    let vart = mel_empty_var_type();
    scope = scope.insert("req", Type::Struct(mel_req_type(ht, mel_uri_type())));
    scope = scope.insert(MEL_USER_SET_VARIABLE_STRUCT_NAME, Type::Struct(vart));
    scope
}

macro_rules! add_builtin_function_type_to_scope {
    ($scope:ident, $builtin:ident) => {
        $scope.insert(
            &$builtin.name(),
            Type::Function(
                $builtin.name(),
                $builtin.return_type_calculator(),
                $builtin.params_type_checker(),
            ),
        )
    };
}

/// Create a scope that contains the types of the MEL builtin functions.
pub fn builtin_function_types() -> Scope<Type> {
    let path_element = Path_ElementBuiltin {};
    let path_elements = Path_ElementsBuiltin {};
    let mtch = MatchBuiltin {};
    let match_replace = Match_ReplaceBuiltin {};
    let add_query = Add_QueryBuiltin {};
    let add_query_multi = Add_Query_MultiBuiltin {};
    let remove_query = Remove_QueryBuiltin {};
    let remove_query_multi = Remove_Query_MultiBuiltin {};
    let keep_query_multi = Keep_Query_MultiBuiltin {};
    let lower = LowerBuiltin {};
    let upper = UpperBuiltin {};

    // Builtin conversion functions.
    let boolean = BooleanBuiltin {};
    let integer = IntegerBuiltin {};
    let unsigned = UnsignedBuiltin {};
    let string = StringBuiltin {};
    let real = RealBuiltin {};

    let mut scopes = Scope::<Type>::default();
    scopes = add_builtin_function_type_to_scope!(scopes, path_element);
    scopes = add_builtin_function_type_to_scope!(scopes, path_elements);
    scopes = add_builtin_function_type_to_scope!(scopes, mtch);
    scopes = add_builtin_function_type_to_scope!(scopes, match_replace);
    scopes = add_builtin_function_type_to_scope!(scopes, add_query);
    scopes = add_builtin_function_type_to_scope!(scopes, add_query_multi);
    scopes = add_builtin_function_type_to_scope!(scopes, remove_query);
    scopes = add_builtin_function_type_to_scope!(scopes, remove_query_multi);
    scopes = add_builtin_function_type_to_scope!(scopes, keep_query_multi);
    scopes = add_builtin_function_type_to_scope!(scopes, lower);
    scopes = add_builtin_function_type_to_scope!(scopes, upper);

    scopes = add_builtin_function_type_to_scope!(scopes, boolean);
    scopes = add_builtin_function_type_to_scope!(scopes, integer);
    scopes = add_builtin_function_type_to_scope!(scopes, real);
    scopes = add_builtin_function_type_to_scope!(scopes, string);
    scopes = add_builtin_function_type_to_scope!(scopes, unsigned);
    scopes
}

/// Create a scope that contains the types of only the MEL core variables present in an HTTP request.
impl<A> From<http::Request<A>> for Scope<Type> {
    fn from(value: http::Request<A>) -> Self {
        // Set up the built-in variables for type checking.
        let mut scope = Scope::<Type>::default();

        let ht = mel_header_type_from_req(&value);
        let urit = mel_uri_type();
        let reqs = mel_req_type(ht, urit);

        // Add those types to the scope.
        scope = scope.insert("req", Type::Struct(reqs));

        scope
    }
}

#[cfg(test)]
mod test_struct {
    use regex::Regex;

    use crate::mel::types::{Struct, Type};

    #[test]
    fn test_wild_struct_field() {
        let mut s = Struct::new("s");

        s.insert_field("testing", Type::String);
        s.insert_wild_field(
            &Regex::new("w.*").expect("Could not compile testing regex"),
            Type::Integer,
        );

        let found = s
            .get_field("testing")
            .expect("Could not find testing field");
        assert_eq!(found, Type::String);

        let found = s.get_field("wild").expect("Could not find wild field");
        assert_eq!(found, Type::Integer);
    }
}
