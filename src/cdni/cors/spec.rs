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

//! CORS (and related) metadata definitions

use crate::macros::IntoCdniMetadata;
use brooks_macros::CdniMetadata;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlAllowOriginPattern {
    pub pattern: String,

    #[serde(skip)]
    pub aug: Option<Regex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlAllowOrigin<A: Debug + Clone + Default> {
    #[serde(rename = "allow-list")]
    pub allow_origin: Vec<AccessControlAllowOriginPattern>,
    #[serde(rename = "wildcard-return")]
    pub wildcard_return: bool,

    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossoriginPolicy<A: Debug + Clone + Default> {
    #[serde(rename = "allow-origin")]
    pub allow_origin: AccessControlAllowOrigin<A>,
    #[serde(rename = "preflight-only")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight_only: Option<bool>,

    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, CdniMetadata)]
pub struct TypedCrossoriginPolicy<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: CrossoriginPolicy<A>,
}

#[cfg(test)]
mod test_parse_cors_metadata {
    use std::path::Path;

    use crate::{cdni::cors::spec::TypedCrossoriginPolicy, tests::read_test_file};

    #[test]
    fn test_verify_draft_figure1() {
        let json = read_test_file(Path::new("./src/cdni/tests/cors/figure1.json"));
        let cop_metadata = serde_json::from_str::<TypedCrossoriginPolicy<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            cop_metadata.tpe,
            TypedCrossoriginPolicy::<()>::typed_cdni_metadata_name()
        );
    }
}
