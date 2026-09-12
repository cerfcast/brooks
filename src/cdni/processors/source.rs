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

//! The Source Metadata Information Implementation

use http::Response;
use reqwest::{
    Body,
    dns::{Addrs, Resolve},
};

use crate::cdni::{
    processing::{
        self, MetadataProcessingAnalysisContext, MetadataProcessingAnalysisError,
        MetadataProcessingAnalysisResult, MetadataProcessingAnalyzed, MetadataProcessingAnalyzer,
        MetadataProcessingInterpreterContext, MetadataProcessingInterpreterError,
    },
    spec::{self, Source, TypedSource},
};
use std::{fmt::Debug, net::SocketAddr, str::FromStr};

#[derive(Debug)]
struct SourceMetadataAnalyzer {}

#[derive(Debug, Clone)]
pub enum SourceProtocol {
    Http11,
}

impl FromStr for SourceProtocol {
    type Err = MetadataProcessingAnalysisError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "http/1.1" {
            return Ok(SourceProtocol::Http11);
        }
        Err(MetadataProcessingAnalysisError::InvalidMetadata(
            format!("{} is not a valid protocol for Source MI", s).into(),
        ))
    }
}

#[derive(Debug, Clone)]
struct SourceMetadataAnalyzed {
    endpoints: Vec<SocketAddr>,
    protocol: SourceProtocol,
}

impl Resolve for SourceMetadataAnalyzed {
    // When connecting to the source represented by this instance
    // of SourceMetadataAnalyzed, always use the endpoints to resolve
    // the name.
    fn resolve(&self, _: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let r: Addrs = Box::new(self.endpoints.clone().into_iter());
        Box::pin(std::future::ready(Ok(r)))
    }
}

impl MetadataProcessingAnalyzed for SourceMetadataAnalyzed {
    fn interpret(
        &self,
        input: processing::MetadataProcessingInterpreterContext,
    ) -> processing::MetadataProcessingInterpretResult {
        let request = match input.request {
            Some(r) => r.clone(),
            None => {
                return Err(MetadataProcessingInterpreterError::RuntimeError(
                    "Missing Input".into(),
                ));
            }
        };

        let client = reqwest::Client::builder()
            .dns_resolver(self.clone())
            .build()
            .map_err(|e| MetadataProcessingInterpreterError::RuntimeError(e.into()))?;
        let request = reqwest::Request::try_from(request)
            .map_err(|e| MetadataProcessingInterpreterError::RuntimeError(e.into()))?;
        let r = input
            .runtime
            .block_on(async { client.execute(request).await })
            .map_err(|e| MetadataProcessingInterpreterError::RuntimeError(e.into()))?;
        let r = Into::<Response<Body>>::into(r).map(|f| f.as_bytes().unwrap_or_default().to_vec());

        Ok(MetadataProcessingInterpreterContext {
            request: None,
            response: Some(r),
            runtime: input.runtime,
        })
    }
}

impl MetadataProcessingAnalyzer for SourceMetadataAnalyzer {
    fn analyze(
        &self,
        _v: &spec::TypedGenericMetadata<()>,
        input: MetadataProcessingAnalysisContext,
    ) -> MetadataProcessingAnalysisResult {
        if _v.tpe != TypedSource::<()>::typed_generic_metadata_name() {
            return Err(MetadataProcessingAnalysisError::InvalidType(
                TypedSource::<()>::typed_generic_metadata_name(),
                _v.tpe.clone(),
            ));
        }

        let typed_source: Source<()> = serde_json::from_value(_v.value.clone())
            .map_err(|e| MetadataProcessingAnalysisError::InvalidMetadata(e.into()))?;

        let typed_source_protocol: SourceProtocol =
            typed_source.protocol.parse().map_err(|_| {
                MetadataProcessingAnalysisError::InvalidMetadata(
                    "Source MI has bad protocol".into(),
                )
            })?;

        // Now, try to parse the endpoints -- they must be IP addresses at this point.

        let mut endpoints: Vec<SocketAddr> = vec![];
        for ep in typed_source.endpoints {
            let address: SocketAddr = ep.parse().map_err(|_| {
                MetadataProcessingAnalysisError::InvalidMetadata(
                    format!("Could not parse {} into IP:port combination", ep).into(),
                )
            })?;
            endpoints.push(address);
        }
        Ok((
            input,
            Box::new(SourceMetadataAnalyzed {
                endpoints,
                protocol: typed_source_protocol,
            }),
        ))
    }
}

#[cfg(test)]
mod processor_tests {
    use std::assert_matches;

    use crate::cdni::{
        processing::{
            MetadataProcessingAnalysisContext, MetadataProcessingAnalysisError,
            MetadataProcessingAnalyzer, MetadataProcessingInterpreterContext,
        },
        processors::source::SourceMetadataAnalyzer,
        tests::test_helpers::generic_source,
    };

    #[test]
    fn source_analysis() {
        let srcv = generic_source(vec!["192.168.0.1:80", "[::1]:80"], "http/1.1");
        let sma = SourceMetadataAnalyzer {};
        let input = MetadataProcessingAnalysisContext {};

        assert!(sma.analyze(&srcv, input).is_ok());
    }

    #[test]
    fn source_interpreter() {
        let srcv = generic_source(
            vec!["172.66.147.243:80", "[2606:4700:10::6814:179a]:80"],
            "http/1.1",
        );
        let sma = SourceMetadataAnalyzer {};
        let input = MetadataProcessingAnalysisContext {};

        let result = sma.analyze(&srcv, input).expect("TODO");

        let interpreter = result.1;

        let rt = tokio::runtime::Runtime::new().expect("Could not get runtime for testing");
        let ic = MetadataProcessingInterpreterContext {
            request: Some(
                http::Request::get("http://www.example.com")
                    .body("".to_string().into())
                    .expect("Could not make basic HTTP request"),
            ),
            response: None,
            runtime: rt,
        };

        let result = interpreter.interpret(ic);
        assert!(result.is_ok())
    }

    #[test]
    fn source_endpoint_not_ip_address() {
        let srcv = generic_source(vec!["192.168.0.1:80", "http://www.example.com"], "http/1.1");
        let sma = SourceMetadataAnalyzer {};
        let input = MetadataProcessingAnalysisContext {};

        assert_matches!(
            sma.analyze(&srcv, input),
            Err(MetadataProcessingAnalysisError::InvalidMetadata(s)) if (*s).to_string() == "Could not parse http://www.example.com into IP:port combination");
    }

    #[test]
    fn source_bad_protocol() {
        let srcv = generic_source(vec!["192.168.0.1", "http://www.example.com"], "http/1.");
        let sma = SourceMetadataAnalyzer {};
        let input = MetadataProcessingAnalysisContext {};

        assert_matches!(
            sma.analyze(&srcv, input),
            Err(MetadataProcessingAnalysisError::InvalidMetadata(_))
        );
    }
}
