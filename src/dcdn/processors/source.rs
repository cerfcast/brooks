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

//! The Source Generic Metadata Processing Implementation

use http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::dns::{Addrs, Resolve};

use crate::{
    cdni::original::spec::TypedSourceMetadata,
    dcdn::{
        imdi,
        interpret::{Error, MetadataInformationResultElements, ProcessedRequestResponse, Stages},
        processors::SimpleProcessorsInterpreterContext,
    },
    processing::interpret::{self, InterpretationResult},
    prr::{self, Prr},
    verifying::original::verify::SourceVerificationKey,
};

use std::net::SocketAddr;
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Clone)]
pub struct SourceMetadataResolver {
    pub addr: SocketAddr,
}

impl Resolve for SourceMetadataResolver {
    fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let addrs: Addrs = Box::new(vec![self.addr].into_iter());
        Box::pin(std::future::ready(Ok(addrs)))
    }
}

mod non_std_http_hdrs {
    use http::HeaderName;
    pub static KEEP_ALIVE: HeaderName = HeaderName::from_static("keep-alive");
    pub static PUBLIC: HeaderName = HeaderName::from_static("public");
}

/// The HTTP hop-by-hop headers
///
/// See [RFC9110](https://www.rfc-editor.org/info/rfc9110/#section-7.6.1).
static HOP_BY_HOP_NAMES: &[&HeaderName] = &[
    &http::header::CONNECTION,
    &non_std_http_hdrs::KEEP_ALIVE,
    &non_std_http_hdrs::PUBLIC,
    &http::header::TE,
    &http::header::TRANSFER_ENCODING,
    &http::header::UPGRADE,
];

/// Filter the hop-by-hop headers that may be in a response from the uCDN.
fn filter_hop_by_hop_headers_from_response(headers: &HeaderMap) -> HeaderMap {
    let mut filtered_header_map = HeaderMap::<HeaderValue>::new();

    // Find the connection headers, if they are present.
    let connection_headers = headers
        .iter()
        .filter_map(|(n, v)| {
            if n == http::header::CONNECTION {
                Some(v.to_str().unwrap_or_default().split(","))
            } else {
                None
            }
        })
        .flatten()
        .map(|f| f.trim())
        .filter_map(|v| http::HeaderName::from_str(v).ok())
        .collect::<Vec<_>>();

    // Now, filter out those headers that were mentioned in CONNECTION or are considered Hop-by-hop.
    for (h, v) in headers {
        if !HOP_BY_HOP_NAMES.contains(&h) && !connection_headers.contains(h) {
            filtered_header_map.insert(h, v.clone());
        }
    }

    filtered_header_map
}

/// Headers that should not be proxied when accessing source from uCDN.
static EDGE_HEADERS: &[&HeaderName] = &[&http::header::ACCEPT_ENCODING];

/// Filter the headers that should not be proxied when accessing source from uCDN.
fn filter_edge_headers_from_request(headers: &HeaderMap) -> HeaderMap {
    let mut filtered_header_map = HeaderMap::<HeaderValue>::new();
    for (n, h) in headers.iter().filter(|(n, _)| !EDGE_HEADERS.contains(n)) {
        filtered_header_map.insert(n, h.clone());
    }

    filtered_header_map
}

impl
    interpret::Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (imdi::ResultValue, MetadataInformationResultElements),
        Stages,
        Error,
    > for TypedSourceMetadata<SourceVerificationKey>
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (imdi::ResultValue, MetadataInformationResultElements),
        Error,
    > {
        let req = &*input.req;

        let mut req: reqwest::Request = req
            .try_into()
            .map_err(|e: prr::Error| Error::InvalidInput(e.into()))?;

        *req.headers_mut() = filter_edge_headers_from_request(req.headers());

        let req_method = req.method().clone();
        let req_url = req.url().clone();

        let mut clientb = reqwest::Client::builder();

        let resolver = SourceMetadataResolver {
            addr: *self
                .value
                .aug
                .sources
                .first()
                .ok_or(Error::AssertionFailure(
                    "Missing sources".to_string().into(),
                ))?
                .0
                .first()
                .ok_or(Error::AssertionFailure(
                    "Missing endpoints".to_string().into(),
                ))?,
        };

        clientb = clientb.dns_resolver(resolver);

        let client = clientb.build().map_err(|e| Error::RuntimeError(e.into()))?;

        let result: Result<ProcessedRequestResponse, Error> = input.runtime.block_on(async {
            let r = client
                .execute(req)
                .await
                .map_err(|e| Error::RuntimeError(e.into()))?;

            let mut result = ProcessedRequestResponse::new_response(&req_url, &req_method);

            result.oheaders = filter_hop_by_hop_headers_from_response(r.headers());

            result
                .set_status(&r.status().as_u16())
                .map_err(|_| Error::RuntimeError(prr::Error::BadStatus.into()))?;

            let b = r.bytes().await.map_err(|e| Error::RuntimeError(e.into()))?;
            result.obody = b.to_vec();
            Ok(result)
        });

        Ok((
            input.with_res(Box::new(result?)),
            (None, MetadataInformationResultElements::default()),
        ))
    }

    fn at(&self, stage: Stages) -> bool {
        stage == Stages::Source
    }
}

#[cfg(test)]
mod test_filter_hop_by_hop_headers_from_response {
    use http::{HeaderMap, HeaderValue};

    use crate::dcdn::processors::source::{
        filter_hop_by_hop_headers_from_response,
        non_std_http_hdrs::{KEEP_ALIVE, PUBLIC},
    };

    #[test]
    fn filter_hop_by_hop() {
        let mut header = HeaderMap::<HeaderValue>::new();

        // Should remove.
        header.insert(
            http::header::TRANSFER_ENCODING,
            HeaderValue::from_static("compress"),
        );
        header.insert(http::header::TE, HeaderValue::from_static("identity"));
        header.insert(
            http::header::CONNECTION,
            HeaderValue::from_static("unknown"),
        );
        header.insert(
            KEEP_ALIVE.clone(),
            HeaderValue::from_static("unknown=value"),
        );
        header.insert(PUBLIC.clone(), HeaderValue::from_static("token"));
        header.insert(http::header::UPGRADE, HeaderValue::from_static("websocket"));

        // Should keep.
        header.insert(http::header::HOST, HeaderValue::from_static("cnn.com"));
        header.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/json"),
        );

        let result = filter_hop_by_hop_headers_from_response(&header);

        assert_eq!(result.len(), 2);
        assert_eq!(
            result
                .values()
                .next()
                .expect("Could not get expected 0th retained header value"),
            "cnn.com"
        );

        assert_eq!(
            result
                .values()
                .nth(1)
                .expect("Could not get expected 1th retained header value"),
            "text/json"
        );
    }

    #[test]
    fn filter_connection_listed_headers() {
        let mut header = HeaderMap::<HeaderValue>::new();

        // Should remove.
        header.insert(
            http::header::CONNECTION,
            HeaderValue::from_static("HOST, CONTENT-TYPE"),
        );

        // Should normally keep -- but according to the semantics of CONNECTION,
        // we do otherwise..
        header.insert(http::header::HOST, HeaderValue::from_static("cnn.com"));
        header.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/json"),
        );

        let result = filter_hop_by_hop_headers_from_response(&header);

        assert_eq!(result.len(), 0);
    }
}
