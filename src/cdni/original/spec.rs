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

//! Definitions of metadata specified in original interconnection specification

use crate::macros::IntoCdniMetadata;
use brooks_macros::CdniMetadata;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub endpoints: Vec<String>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetadata<A: Debug + Clone + Default> {
    pub sources: Vec<Source>,
    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, CdniMetadata)]
pub struct TypedSourceMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: SourceMetadata<A>,
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

#[cfg(test)]
mod test_parse_source_metadata {
    use std::path::Path;

    use crate::{cdni::original::spec::TypedSourceMetadata, tests::read_test_file};

    #[test]
    fn test_verify_8006_example() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/source/modified-8006-example.json",
        ));
        let cop_metadata = serde_json::from_str::<TypedSourceMetadata<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            cop_metadata.tpe,
            TypedSourceMetadata::<()>::typed_cdni_metadata_name()
        );
    }
}
