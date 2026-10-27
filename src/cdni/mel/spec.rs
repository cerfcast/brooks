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

//! MEL metadata definitions

use crate::macros::IntoCdniMetadata;
use brooks_macros::CdniMetadata;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVariable<A: Debug + Clone + Default> {
    #[serde(rename = "variable-name")]
    pub variable_name: String,
    #[serde(rename = "variable-value")]
    pub variable_value: String,
    #[serde(rename = "value-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_is_expression: Option<bool>,

    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, CdniMetadata)]
pub struct TypedSetVariable<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: SetVariable<A>,
}
