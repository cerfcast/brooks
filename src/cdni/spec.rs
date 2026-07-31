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

use brooks_macros::TypedGenericMetadata;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedGenericMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: Value,
    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostMetadata<A: Debug + Clone + Default> {
    pub metadata: Vec<TypedGenericMetadata<A>>,
    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedHostMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: HostMetadata<A>,
}

#[cfg(test)]
mod test_parse_host_metadata {
    use std::path::Path;

    use crate::{cdni::spec::TypedHostMetadata, tests::read_test_file};

    #[test]
    fn test_verify_bad_generic_md_typename() {
        let json = read_test_file(Path::new("./src/cdni/tests/host_metadata/all.json"));
        let host_metadata = serde_json::from_str::<TypedHostMetadata<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            host_metadata.tpe,
            TypedHostMetadata::<()>::typed_generic_metadata_name()
        );
    }
}
