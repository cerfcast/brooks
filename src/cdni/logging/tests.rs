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

#[cfg(test)]
mod test_parse_logging_metadata {
    use std::path::Path;

    use crate::{cdni::logging::spec::TypedLoggingMetadata, tests::read_test_file};

    #[test]
    fn test_verify_draft_figure_7_8() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/logging/modified-figure-7-8.json",
        ));
        let logging_metadata = serde_json::from_str::<TypedLoggingMetadata<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            logging_metadata.tpe,
            TypedLoggingMetadata::<()>::typed_cdni_metadata_name()
        );
    }

    #[test]
    fn test_verify_draft_figure_7_8_standard() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/logging/modified-figure-7-8-standard.json",
        ));
        let logging_metadata = serde_json::from_str::<TypedLoggingMetadata<()>>(&json)
            .expect("Could not parse JSON test file");
        assert_eq!(
            logging_metadata.tpe,
            TypedLoggingMetadata::<()>::typed_cdni_metadata_name()
        );
    }
}

#[cfg(test)]
mod test_logging_record {
    use std::path::Path;

    use crate::{cdni::logging::record::LogRecordRaw, tests::read_test_file};

    #[test]
    fn test_serialize_logging_record() {
        let r = LogRecordRaw::default();

        let serialized_record =
            serde_json::to_string_pretty(&r).expect("Could not serialize valid logging record");

        println!("{serialized_record}");
        panic!()
    }

    #[test]
    fn test_parse_draft_figure_5() {
        let json = read_test_file(Path::new("./src/cdni/tests/logging/figure-5.json"));
        let r =
            serde_json::from_str::<LogRecordRaw>(&json).expect("Could not parse JSON test file");

        println!("{r:?}");
        panic!()
    }
}
