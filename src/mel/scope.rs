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

use std::{collections::HashMap, fmt::Debug, ops::Add, sync::Arc};

use http::uri::Scheme;

use crate::mel::{
    interpreter::interpret::{StructValue, TypedValue, Value},
    tvs::{
        Add_Query_MultiBuiltin, Add_QueryBuiltin, BooleanBuiltin, BuiltinFunctionType,
        Keep_Query_MultiBuiltin, Match_ReplaceBuiltin, MatchBuiltin, Path_ElementBuiltin,
        Path_ElementsBuiltin, Remove_Query_MultiBuiltin, Remove_QueryBuiltin,
        Type::{self, Function},
        header_type, header_type_from_req, req_type, uri_type,
    },
};

#[derive(Debug, Clone, Default)]
pub struct Scope<I: Clone + Default> {
    pub items: HashMap<String, I>,
}

impl<I: Clone + Default> Scope<I> {
    pub fn lookup(&self, id: &str) -> Option<I> {
        self.items.get(id).cloned()
    }
    pub fn insert(&self, id: &str, value: I) -> Self {
        let mut next = self.items.clone();
        next.insert(id.to_string(), value);
        Self { items: next }
    }
}

impl<I: Clone + Default> Add for &Scope<I> {
    type Output = Scope<I>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut ns = Scope::<I> {
            items: HashMap::new(),
        };
        for (k, v) in &self.items {
            ns = ns.insert(k, v.clone());
        }
        for (k, v) in &rhs.items {
            ns = ns.insert(k, v.clone());
        }
        ns
    }
}

#[derive(Debug, Clone)]
pub struct Scopes<I: Clone + Default> {
    pub scopes: Vec<Scope<I>>,
}

impl<I: Clone + Default> Scopes<I> {
    pub fn lookup(&self, id: &str) -> Option<I> {
        self.scopes[0].lookup(id)
    }

    pub fn insert(&self, id: &str, value: I) -> Self {
        let updated_scope = self.scopes[0].insert(id, value);

        let mut next = self.scopes.clone();
        next[0] = updated_scope;

        Self { scopes: next }
    }

    pub fn enter(&self) -> Scopes<I> {
        let mut next = self.scopes.clone();
        next.extend([Scope::default()]);
        Self { scopes: next }
    }

    pub fn current(&self) -> &Scope<I> {
        &self.scopes[0]
    }
}

impl<I: Clone + Default> Default for Scopes<I> {
    fn default() -> Self {
        Self {
            scopes: vec![Scope::default()],
        }
    }
}

/// Create a scope that contains the types of the MEL core variables.
pub fn minimal_core_variable_types() -> Scope<Type> {
    let mut scope = Scope::<Type>::default();
    let ht = header_type(true);
    scope = scope.insert("req", Type::Struct(req_type(ht, uri_type())));
    scope
}

macro_rules! add_builtin_function_type_to_scope {
    ($scope:ident, $builtin:ident) => {
        $scope.insert(
            &$builtin.name(),
            Function(Arc::new($builtin.return_type()), $builtin.parameters()),
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
    let boolean = BooleanBuiltin {};

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
    scopes = add_builtin_function_type_to_scope!(scopes, boolean);
    scopes
}

/// Create a scope that contains the types of only the MEL core variables present in an HTTP request.
impl<A> From<http::Request<A>> for Scope<Type> {
    fn from(value: http::Request<A>) -> Self {
        // Set up the built-in variables for type checking.
        let mut scope = Scope::<Type>::default();

        let ht = header_type_from_req(&value);
        let urit = uri_type();
        let reqs = req_type(ht, urit);

        // Add those types to the scope.
        scope = scope.insert("req", Type::Struct(reqs));

        scope
    }
}

/// Create a scope that contains the values of only the MEL core variables present in an HTTP request.
impl<A> From<http::Request<A>> for Scope<TypedValue> {
    fn from(value: http::Request<A>) -> Self {
        let ht = header_type_from_req(&value);
        let urit = uri_type();
        let reqt = req_type(ht.clone(), urit.clone());

        // Set up the built-in variables for interpreting.
        let mut value_scope = Scope::<TypedValue>::default();

        let mut reqv = StructValue::new(reqt.clone());

        let mut hv = StructValue::new(ht.clone());

        value.headers().iter().for_each(|header| {
            if let Ok(x) = header.1.to_str() {
                hv.insert_field(
                    &header.0.to_string().replace("-", "_").to_lowercase(),
                    TypedValue {
                        value: Value::String(x.to_string()),
                        tipe: Type::String,
                    },
                )
                .expect("header field value is mistyped");
            }
        });

        let mut uriv = StructValue::new(urit.clone());

        uriv.insert_field(
            "path",
            TypedValue {
                value: Value::String(value.uri().path().to_string()),
                tipe: Type::String,
            },
        )
        .expect("path field value is mistyped.");

        uriv.insert_field(
            "query",
            TypedValue {
                value: Value::String(value.uri().query().unwrap_or_default().to_string()),
                tipe: Type::String,
            },
        )
        .expect("query field value is mistyped.");

        reqv.insert_field(
            "h",
            TypedValue {
                value: Value::Struct(hv),
                tipe: Type::Struct(ht.clone()),
            },
        )
        .expect("h field value is mistyped.");

        reqv.insert_field(
            "uri",
            TypedValue {
                value: Value::Struct(uriv),
                tipe: Type::Struct(urit.clone()),
            },
        )
        .expect("uri field value is mistyped.");

        reqv.insert_field(
            "method",
            TypedValue {
                value: Value::String(value.method().to_string()),
                tipe: Type::String,
            },
        )
        .expect("method field value is mistyped.");

        reqv.insert_field(
            "scheme",
            TypedValue {
                value: Value::String(value.uri().scheme().unwrap_or(&Scheme::HTTP).to_string()),
                tipe: Type::String,
            },
        )
        .expect("Header field value is mistyped.");

        value_scope = value_scope.insert(
            "req",
            TypedValue {
                value: Value::Struct(reqv),
                tipe: Type::Struct(reqt),
            },
        );

        value_scope
    }
}

#[cfg(test)]
mod scope_tests {
    use crate::mel::scope::Scope;
    use std::assert_matches;
    use std::collections::HashMap;

    #[test]
    fn test_operator_plus() {
        let mut s1 = Scope::<i8> {
            items: HashMap::new(),
        };
        s1 = s1.insert("x", 5);
        let mut s2 = Scope::<i8> {
            items: HashMap::new(),
        };
        s2 = s2.insert("y", 4);

        let s3 = &s1 + &s2;

        assert_matches!(s3.lookup("y"), Some(4));
        assert_matches!(s3.lookup("x"), Some(5));
    }
}
