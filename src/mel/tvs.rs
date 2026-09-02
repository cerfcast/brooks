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

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
};

use brooks_macros::builtin_function;
use regex::Regex;

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
pub struct Params {
    pub args: Vec<Type>,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum Type {
    Boolean,
    Integer,
    String,
    Regex,
    IPAddress,
    Params(Params),
    Function(Arc<Type>, Params),
    Struct(Struct),
    #[default]
    None,
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
            Type::Boolean => write!(f, "Bool"),
            Type::Integer => write!(f, "Integer"),
            Type::String => write!(f, "String"),
            Type::Regex => write!(f, "Regex"),
            Type::IPAddress => write!(f, "IPAddress"),
            Type::Params(items) => write!(
                f,
                "Parameters: {}",
                items
                    .args
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Type::Function(result, args) => write!(
                f,
                "Return Type: {result}, Argument Types: {}",
                args.args
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Type::Struct(s) => write!(f, "Struct: {s}"),
            Type::None => write!(f, "None"),
        }
    }
}

impl Struct {
    pub fn type_for_field(&self, field_name: &str) -> Option<Type> {
        self.fields.get(field_name).cloned()
    }
}

pub trait BuiltinFunctionType: Debug {
    fn name(&self) -> String;
    fn parameters(&self) -> Params;
    fn return_type(&self) -> Type;
}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::Integer)]
pub struct Path_ElementBuiltin {}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::Integer, Type::Integer)]
pub struct Path_ElementsBuiltin {}

#[derive(Debug, Clone)]
#[builtin_function(Type::Boolean, Type::Integer)]
pub struct BooleanBuiltin {}

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

pub(crate) fn header_type(wild: bool) -> Struct {
    let mut ht = Struct::new("h");
    if wild {
        ht.insert_wild_field(
            &Regex::new(".*").expect("Could not compile wildcard regex for header type"),
            Type::String,
        );
    }

    ht
}

pub(crate) fn header_type_from_req<A>(value: &http::Request<A>) -> Struct {
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

pub(crate) fn uri_type() -> Struct {
    // Make the URI type.
    let mut urit = Struct::new("uri");
    urit.insert_field("path", Type::String);
    urit.insert_field("query", Type::String);

    urit
}

pub(crate) fn req_type(header_type: Struct, uri_type: Struct) -> Struct {
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

#[cfg(test)]
mod test_struct {
    use regex::Regex;

    use crate::mel::tvs::{Struct, Type};

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
