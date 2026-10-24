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
    io,
    net::IpAddr,
    sync::Arc,
};

use regex::Regex;

use crate::{
    environment::scope::Scope,
    mel::{
        interpreter::interpret::{BuiltinFunction, MelInterpError},
        types::{
            mel_header_type_from_processable_rr, mel_header_type_from_req, mel_req_type,
            mel_uri_type,
        },
    },
    prr,
};

use crate::mel::{
    analysis::CompiledConstant,
    types::{Struct, Type},
};

#[derive(Debug, Clone)]
pub struct StructValue {
    pub fields: HashMap<String, TypedValue>,
    pub tpe: Struct,
}

impl StructValue {
    pub fn new(tpe: Struct) -> StructValue {
        StructValue {
            fields: HashMap::new(),
            tpe,
        }
    }

    pub fn insert_field(
        &mut self,
        name: &str,
        value: TypedValue,
    ) -> Result<(), Box<MelInterpError>> {
        let ft = self
            .tpe
            .get_field(name)
            .ok_or(MelInterpError::UnknownField(name.to_string()))?;

        if ft != value.tpe {
            return Err(
                MelInterpError::MistypedField(name.to_string(), ft.clone(), value.tpe).into(),
            );
        }

        self.fields.insert(name.to_string(), value);
        Ok(())
    }
}

impl Display for StructValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut field_values = self
            .fields
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>();
        field_values.sort();
        let field_values = field_values.join(", ");
        write!(
            f,
            "Type: {}, Field Values: {}",
            self.tpe.name,
            if !field_values.is_empty() {
                field_values
            } else {
                "None".to_string()
            }
        )
    }
}

#[derive(Default, Debug, Clone)]
pub enum Value {
    Integer(i64),
    Unsigned(u64),
    Real(f64),
    String(String),
    Boolean(bool),
    Regex(Regex),
    IPAddress(IpAddr),
    Function(Arc<dyn BuiltinFunction>),
    Struct(StructValue),
    ArgumentList(Vec<TypedValue>),
    Nil,
    #[default]
    Uninitialized,
}

#[derive(Debug, Clone)]
pub enum TypedValueError {
    ConversionError(String),
}

impl Display for TypedValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedValueError::ConversionError(ce) => write!(f, "Conversion error: {ce}"),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct TypedValue {
    pub value: Value,
    pub tpe: Type,
}

impl Display for TypedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Value::Nil => write!(f, "nil"),
            Value::Integer(i) => write!(f, "{i}"),
            Value::Unsigned(u) => write!(f, "{u}"),
            Value::Real(i) => write!(f, "{i}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Boolean(b) => write!(f, "{b}"),
            Value::Regex(regex) => write!(f, "{regex}"),
            Value::IPAddress(ip_addr) => write!(f, "{ip_addr}"),
            Value::Function(bft) => write!(f, "Function: {}", bft.name()),
            Value::Struct(struct_value) => write!(f, "{}", struct_value),
            Value::ArgumentList(typed_values) => write!(
                f,
                "Argument List: {}",
                typed_values
                    .iter()
                    .map(|tv| { tv.to_string() })
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Value::Uninitialized => write!(f, "Uninitialized"),
        }
    }
}

impl From<&CompiledConstant> for TypedValue {
    fn from(value: &CompiledConstant) -> Self {
        match value {
            CompiledConstant::Integer(i) => TypedValue {
                value: Value::Integer(*i),
                tpe: Type::Integer,
            },
            CompiledConstant::String(s) => TypedValue {
                value: Value::String(s.clone()),
                tpe: Type::String,
            },
            CompiledConstant::Boolean(b) => TypedValue {
                value: Value::Boolean(*b),
                tpe: Type::Boolean,
            },
            CompiledConstant::IPAddress(ip) => TypedValue {
                value: Value::IPAddress(*ip),
                tpe: Type::IPAddress,
            },
        }
    }
}

impl TryFrom<http::Request<Vec<u8>>> for Scope<TypedValue> {
    type Error = io::Error;
    fn try_from(value: http::Request<Vec<u8>>) -> Result<Self, Self::Error> {
        let ht = mel_header_type_from_req(&value);
        let urit = mel_uri_type();
        let reqt = mel_req_type(ht.clone(), urit.clone());

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
                        tpe: Type::String,
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
                tpe: Type::String,
            },
        )
        .expect("path field value is mistyped.");

        uriv.insert_field(
            "query",
            TypedValue {
                value: Value::String(value.uri().query().unwrap_or_default().to_string()),
                tpe: Type::String,
            },
        )
        .expect("query field value is mistyped.");

        reqv.insert_field(
            "h",
            TypedValue {
                value: Value::Struct(hv),
                tpe: Type::Struct(ht.clone()),
            },
        )
        .expect("h field value is mistyped.");

        reqv.insert_field(
            "uri",
            TypedValue {
                value: Value::Struct(uriv),
                tpe: Type::Struct(urit.clone()),
            },
        )
        .expect("uri field value is mistyped.");

        reqv.insert_field(
            "method",
            TypedValue {
                value: Value::String(value.method().to_string()),
                tpe: Type::String,
            },
        )
        .expect("method field value is mistyped.");

        reqv.insert_field(
            "scheme",
            TypedValue {
                value: Value::String(value.uri().scheme().expect("Missing scheme").to_string()),
                tpe: Type::String,
            },
        )
        .expect("Header field value is mistyped.");

        value_scope = value_scope.insert(
            "req",
            TypedValue {
                value: Value::Struct(reqv),
                tpe: Type::Struct(reqt),
            },
        );

        Ok(value_scope)
    }
}

/// Create a scope that contains the values of the MEL core variables present in an HTTP request.
impl<A> From<&dyn prr::Prr<A>> for Scope<TypedValue> {
    fn from(value: &dyn prr::Prr<A>) -> Self {
        let ht = mel_header_type_from_processable_rr(value);
        let urit = mel_uri_type();
        let reqt = mel_req_type(ht.clone(), urit.clone());

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
                        tpe: Type::String,
                    },
                )
                .expect("header field value is mistyped");
            }
        });

        let mut uriv = StructValue::new(urit.clone());

        uriv.insert_field(
            "path",
            TypedValue {
                value: Value::String(value.url().expect("Missing URI").path().to_string()),
                tpe: Type::String,
            },
        )
        .expect("path field value is mistyped.");

        uriv.insert_field(
            "query",
            TypedValue {
                value: Value::String(
                    value
                        .url()
                        .expect("Missing URI")
                        .query()
                        .unwrap_or_default()
                        .to_string(),
                ),
                tpe: Type::String,
            },
        )
        .expect("query field value is mistyped.");

        reqv.insert_field(
            "h",
            TypedValue {
                value: Value::Struct(hv),
                tpe: Type::Struct(ht.clone()),
            },
        )
        .expect("h field value is mistyped.");

        reqv.insert_field(
            "uri",
            TypedValue {
                value: Value::Struct(uriv),
                tpe: Type::Struct(urit.clone()),
            },
        )
        .expect("uri field value is mistyped.");

        reqv.insert_field(
            "method",
            TypedValue {
                //value: Value::String(value.method().to_string()),
                value: Value::String(http::method::Method::GET.to_string()),
                tpe: Type::String,
            },
        )
        .expect("method field value is mistyped.");

        reqv.insert_field(
            "scheme",
            TypedValue {
                value: Value::String(value.url().expect("Missing URI").scheme().to_string()),
                tpe: Type::String,
            },
        )
        .expect("Header field value is mistyped.");

        value_scope = value_scope.insert(
            "req",
            TypedValue {
                value: Value::Struct(reqv),
                tpe: Type::Struct(reqt),
            },
        );

        value_scope
    }
}
