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

//! All Metadata Information except for what is defined in Processing Stages.

use crate::macros::IntoCdniMetadata;
use brooks_macros::CdniMetadata;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::fmt::Debug;

impl<A: Debug + Clone + Default, T: IntoCdniMetadata> From<T> for TypedGenericMetadata<A> {
    fn from(value: T) -> Self {
        Self {
            tpe: value.metadata_type(),
            value: value.metadata_value(),
            aug: A::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedGenericMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: Value,
    #[serde(skip)]
    pub aug: A,
}

impl<A: Debug + Clone + Default> TypedGenericMetadata<A> {
    pub fn typed_generic_metadata_name() -> String {
        "GenericMetadata".to_string()
    }

    pub fn typed_value<AA: Debug + Default + Clone>(
        sr: TypedGenericMetadata<AA>,
    ) -> TypedGenericMetadata<AA> {
        sr.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source<A: Debug + Clone + Default> {
    pub endpoints: Vec<String>,
    pub protocol: String,
    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, CdniMetadata)]
pub struct TypedSource<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: Source<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy<A: Debug + Clone + Default> {
    pub policy: String,
    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, CdniMetadata)]
pub struct TypedCachePolicy<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: CachePolicy<A>,
}
