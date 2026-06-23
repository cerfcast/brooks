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

use std::{collections::HashMap, fmt::Debug, sync::Arc};

/// Types

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Struct {
    pub name: String,
    pub fields: HashMap<String, Type>,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum Type {
    Boolean,
    Integer,
    String,
    Regex,
    Params(Vec<Type>),
    Function(Arc<Type>, Vec<Type>),
    Struct(Struct),
    #[default]
    None,
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for Struct {
    fn to_string(&self) -> String {
        format!(
            "Name: {}, Fields: {}",
            self.name,
            self.fields.keys().cloned().collect::<Vec<_>>().join(","),
        )
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Boolean => "Bool".into(),
            Type::Integer => "Integer".into(),
            Type::String => "String".into(),
            Type::Regex => "Regex".into(),
            Type::Params(items) => format!(
                "Parameters: {}",
                items
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Type::Function(result, args) => format!(
                "Return Type: {}, Argument Types: {}",
                result.to_string(),
                args.iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Type::Struct(s) => format!("Struct: {}", s.to_string()),
            Type::None => "None".to_string(),
        }
    }
}

impl Struct {
    pub fn type_for_field(&self, field_name: &str) -> Option<Type> {
        self.fields.get(field_name).cloned()
    }
}
