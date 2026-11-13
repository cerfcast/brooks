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

//! The Mel Generic Metadata Processing Implementation

use std::{collections::HashMap, fmt::Display, io::Read, str::FromStr};

use flate2::{Compression, GzBuilder};
use http::{
    HeaderValue,
    header::{ACCEPT_ENCODING, CONTENT_ENCODING},
};
use zlib_rs::{DeflateConfig, ReturnCode, compress_bound, compress_slice};

use crate::{
    cdni::cors::spec::{TypedAllowCompress, TypedCrossoriginPolicy},
    dcdn::{
        error::ImdInterpretError,
        imdi,
        interpret::{
            self,
            Error::{self, NotImplemented},
            MetadataInformationResultElements, Stages,
        },
        processors::SimpleProcessorsInterpreterContext,
    },
    processing::interpret::{InterpretationResult, Interpreter},
    verifying::cors::verify::{AllowCompressVerificationKey, CrossoriginPolicyVerificationKey},
};

impl
    Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (imdi::ResultValue, MetadataInformationResultElements),
        Stages,
        Error,
    > for TypedCrossoriginPolicy<CrossoriginPolicyVerificationKey>
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (imdi::ResultValue, MetadataInformationResultElements),
        Error,
    > {
        if let Some(preflight_only) = self.value.preflight_only
            && preflight_only
        {
            return Err(Error::RuntimeError(
                NotImplemented("Missing response when processing MI.CrossoriginPolicy".into())
                    .into(),
            )
            .into());
        }

        let mut res = match input.res {
            Some(res) => res,
            None => {
                return Err(Error::RuntimeError(
                    Error::AssertionFailure(
                        "Missing response when processing MI.CrossoriginPolicy".into(),
                    )
                    .into(),
                )
                .into());
            }
        };
        let req = input.req;

        let origin_value = match req
            .headers()
            .iter()
            .find(|(name, _)| *name == http::header::ORIGIN)
        {
            Some(origin_value) => origin_value.1.clone(),
            None => todo!(),
        };

        let origin_value = origin_value.to_str().map_err(|e| {
            Error::InvalidInput(
                format!("ORIGIN header value could not be converted to string: {e}").into(),
            )
        })?;

        // See whether the ORIGIN value in the request matches any of the
        // patterns.
        let apply = self
            .value
            .allow_origin
            .allow_origin
            .iter()
            .any(|v| v.aug.as_ref().unwrap().is_match(origin_value));

        // If any of the patterns matched the ORIGIN value,
        // set the ORIGIN header value in the response.
        if apply {
            if self.value.allow_origin.wildcard_return {
                res.headers_mut().insert(
                    http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                );
            } else {
                res.headers_mut().insert(
                    http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_str(origin_value)
                        .map_err(|e| Error::RuntimeError(e.into()))?,
                );
            }
        }

        let updated_context = SimpleProcessorsInterpreterContext {
            scopes: input.scopes,
            mode: input.mode,
            runtime: input.runtime,
            req,
            res: Some(res),
        };

        Ok((
            updated_context,
            (None, MetadataInformationResultElements::default()),
        ))
    }

    fn at(&self, stage: Stages) -> bool {
        stage == Stages::Post
    }
}

#[cfg(test)]
mod test_crossorigin_policy {}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum ContentCodingTypes {
    Identity,
    Compress,
    Gzip,
    Deflate,
}

impl Display for ContentCodingTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentCodingTypes::Identity => write!(f, "identity"),
            ContentCodingTypes::Compress => write!(f, "compress"),
            ContentCodingTypes::Gzip => write!(f, "gzip"),
            ContentCodingTypes::Deflate => write!(f, "deflate"),
        }
    }
}

impl FromStr for ContentCodingTypes {
    type Err = interpret::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("identity") {
            Ok(ContentCodingTypes::Identity)
        } else if s.eq_ignore_ascii_case("compress") {
            Ok(ContentCodingTypes::Compress)
        } else if s.eq_ignore_ascii_case("gzip") {
            Ok(ContentCodingTypes::Gzip)
        } else if s.eq_ignore_ascii_case("deflate") {
            Ok(ContentCodingTypes::Deflate)
        } else {
            Err(Error::RuntimeError(
                ImdInterpretError::MetadataProcessingError(
                    "Unsupported Accept-Encoding method:".into(),
                )
                .into(),
            ))
        }
    }
}

fn interpret_allow_compress<'a>(
    input: SimpleProcessorsInterpreterContext<'a>,
) -> InterpretationResult<
    SimpleProcessorsInterpreterContext<'a>,
    (imdi::ResultValue, MetadataInformationResultElements),
    interpret::Error,
> {
    let mut res = match input.res {
        Some(res) => res,
        None => {
            return Err(Error::RuntimeError(
                Error::AssertionFailure("Missing response when processing MI.AllowCompress".into())
                    .into(),
            )
            .into());
        }
    };

    let req = input.req;

    let accept_encoding_value = req
        .headers()
        .into_iter()
        .find(|(v, _)| **v == ACCEPT_ENCODING)
        .map(|f| f.1);

    // Build up the encoders that are available.
    let deflate_repr = DeflateAcceptCoding {};
    let gzip_repr = GzipAcceptCoding {};
    let identity_repr = IdentityAcceptCoding {};

    let mut encoders: HashMap<ContentCodingTypes, &dyn AcceptCodingRepr> = HashMap::new();

    encoders.insert(deflate_repr.token(), &deflate_repr);
    encoders.insert(gzip_repr.token(), &gzip_repr);
    encoders.insert(identity_repr.token(), &identity_repr);

    let chosen_encoding = if let Some(accept_encoding_value) = accept_encoding_value {
        // Try to parse the Accept-Encodings header.
        let codings =
            TryInto::<AcceptCodings>::try_into(accept_encoding_value.to_str().map_err(|e| {
                Error::RuntimeError(ImdInterpretError::MetadataProcessingError(e.into()).into())
            })?)?;

        let mut chosen_encoding = Some(&identity_repr as &dyn AcceptCodingRepr);
        for (method, q) in &codings.codings {
            // method is the remaining coding method with the highest q score. If
            // it is acceptable to the client and we support it, then let's use it.
            if *q != 0.0
                && let Some(repr) = encoders.get(method)
            {
                chosen_encoding = Some(*repr);
                break;
            }

            // We are not yet scraping the bottom of the barrel.
            if *q != 0.0 {
                continue;
            }

            // When we are nearing the end, we don't consider any coding methods with q values
            // of 0.0. However, if the coding method is identity, then that means the user does
            // not want us to fall back to that.
            if *method == ContentCodingTypes::Identity {
                chosen_encoding = None;
                break;
            }
        }
        chosen_encoding
    } else {
        // If no Accept-Encoding header field is in the request, any content coding is considered acceptable by the user agent.
        Some(&deflate_repr as &dyn AcceptCodingRepr)
    };

    if let Some(repr) = chosen_encoding {
        res.headers_mut().insert(
            CONTENT_ENCODING,
            HeaderValue::from_str(&repr.token().to_string()).map_err(|e| {
                Error::RuntimeError(ImdInterpretError::MetadataProcessingError(e.into()).into())
            })?,
        );

        let body = res.body();
        let reprd = repr.code(body);
        *res.body_mut() = reprd;
    } else {
        // Log.
        todo!()
    }

    let updated_context = SimpleProcessorsInterpreterContext {
        scopes: input.scopes,
        mode: input.mode,
        runtime: input.runtime,
        req,
        res: Some(res),
    };

    Ok((
        updated_context,
        (None, MetadataInformationResultElements::default()),
    ))
}

impl
    Interpreter<
        SimpleProcessorsInterpreterContext<'_>,
        (imdi::ResultValue, MetadataInformationResultElements),
        Stages,
        Error,
    > for TypedAllowCompress<AllowCompressVerificationKey>
{
    fn interpret<'a>(
        &self,
        input: SimpleProcessorsInterpreterContext<'a>,
    ) -> InterpretationResult<
        SimpleProcessorsInterpreterContext<'a>,
        (imdi::ResultValue, MetadataInformationResultElements),
        Error,
    > {
        interpret_allow_compress(input)
    }

    fn at(&self, stage: Stages) -> bool {
        stage == Stages::Post
    }
}

type AcceptCoding = (ContentCodingTypes, f64);

#[derive(Debug, Clone, Default)]
struct AcceptCodings {
    pub codings: Vec<AcceptCoding>,
}

impl Display for AcceptCodings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.codings
                .iter()
                .map(|(coding, q)| { format!("{coding};q={q}") })
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

/// What is required to be considered able to encode according to an HTTP content coding.
///
/// See [RFC9110](https://www.rfc-editor.org/info/rfc9110/#name-accept-encoding) and
/// the [HTTP Content Coding Registry](https://www.iana.org/assignments/http-parameters).
trait AcceptCodingRepr {
    fn code(&self, i: &[u8]) -> Vec<u8>;
    fn token(&self) -> ContentCodingTypes;
}

/// Deflate HTTP Content Coding Method Implementation.
struct DeflateAcceptCoding {}
impl AcceptCodingRepr for DeflateAcceptCoding {
    fn code(&self, i: &[u8]) -> Vec<u8> {
        let mut compressed_body: Vec<u8> = vec![0u8; compress_bound(i.len())];

        let (_, rc) = compress_slice(&mut compressed_body, i, DeflateConfig::default());

        if rc != ReturnCode::Ok {
            todo!()
        }

        compressed_body
    }

    fn token(&self) -> ContentCodingTypes {
        ContentCodingTypes::Deflate
    }
}

/// Gzip HTTP Content Coding Method Implementation.
struct GzipAcceptCoding {}
impl AcceptCodingRepr for GzipAcceptCoding {
    fn code(&self, i: &[u8]) -> Vec<u8> {
        let mut gzipper = GzBuilder::new().read(i, Compression::best());
        let mut compressed_body: Vec<u8> = vec![];

        gzipper.read_to_end(&mut compressed_body).expect("TODO");

        compressed_body
    }

    fn token(&self) -> ContentCodingTypes {
        ContentCodingTypes::Gzip
    }
}

/// Identity (no op) HTTP Content Coding Method Implementation.
struct IdentityAcceptCoding {}
impl AcceptCodingRepr for IdentityAcceptCoding {
    fn code(&self, i: &[u8]) -> Vec<u8> {
        i.to_vec()
    }

    fn token(&self) -> ContentCodingTypes {
        ContentCodingTypes::Identity
    }
}

impl TryFrom<&str> for AcceptCodings {
    type Error = interpret::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result: Vec<(ContentCodingTypes, f64)> = vec![];
        for coding in value.split(",") {
            let (coding, q) = match coding.split_once(";") {
                Some((coding, q)) => (coding.trim(), Some(q.trim())),
                None => (coding.trim(), None),
            };

            let q = if let Some(q) = q {
                if let Some(q) = q.strip_prefix("q=") {
                    q.parse::<f64>().map_err(|_| {
                        Error::RuntimeError(
                            ImdInterpretError::MetadataProcessingError(
                                "Bad Accept-Encoding header value:".into(),
                            )
                            .into(),
                        )
                    })?
                } else {
                    return Err(Error::RuntimeError(
                        ImdInterpretError::MetadataProcessingError(
                            "Bad Accept-Encoding header value:".into(),
                        )
                        .into(),
                    ));
                }
            } else {
                1.0f64
            };

            // Any accept encoding coding methods that do not parse, we skip.
            if let Ok(coding) = coding.parse::<ContentCodingTypes>() {
                result.push((coding, q))
            }
        }

        // Sort so that the codings with the higher q are earlier in the list.
        result.sort_by(|l, r| l.1.total_cmp(&r.1).reverse());

        Ok(AcceptCodings { codings: result })
    }
}

#[cfg(test)]
mod test_allow_compress {
    use reqwest::ResponseBuilderExt;

    use crate::{
        dcdn::{
            imdi::InterpretMode,
            processors::{
                SimpleProcessorsInterpreterContext,
                cors::{AcceptCodings, ContentCodingTypes, interpret_allow_compress},
            },
        },
        utils::try_to_url,
    };

    #[test]
    fn test_simple_accept_encoding_pass() {
        let result = TryInto::<AcceptCodings>::try_into("compress, gzip")
            .expect("Could not parse a valid Accept-Encoding value");
        assert_eq!(format!("{result}"), "compress;q=1,gzip;q=1");
        assert_eq!(result.codings.len(), 2);
        assert_eq!(result.codings[0], (ContentCodingTypes::Compress, 1.0));
        assert_eq!(result.codings[1], (ContentCodingTypes::Gzip, 1.0));

        let result = TryInto::<AcceptCodings>::try_into("compress; q=5, gzip")
            .expect("Could not parse a valid Accept-Encoding value");
        assert_eq!(format!("{result}"), "compress;q=5,gzip;q=1");
        assert_eq!(result.codings.len(), 2);
        assert_eq!(result.codings[0], (ContentCodingTypes::Compress, 5.0));
        assert_eq!(result.codings[1], (ContentCodingTypes::Gzip, 1.0));

        let result = TryInto::<AcceptCodings>::try_into("compress, gzip; q=0.5")
            .expect("Could not parse a valid Accept-Encoding value");
        assert_eq!(result.codings.len(), 2);
        assert_eq!(result.codings[0], (ContentCodingTypes::Compress, 1.0));
        assert_eq!(result.codings[1], (ContentCodingTypes::Gzip, 0.5));

        let result = TryInto::<AcceptCodings>::try_into("compress; q=0.0, gzip; q=0.5")
            .expect("Could not parse a valid Accept-Encoding value");
        assert_eq!(result.codings.len(), 2);
        assert_eq!(result.codings[0], (ContentCodingTypes::Gzip, 0.5));
        assert_eq!(result.codings[1], (ContentCodingTypes::Compress, 0.0));
    }

    fn build_test_request_response(
        acceptable_encodings: Option<&str>,
    ) -> (
        Box<http::request::Request<Vec<u8>>>,
        http::response::Response<Vec<u8>>,
    ) {
        let req = http::request::Builder::new();
        let mut req = req.uri("https://www.example.com");

        req = if let Some(acceptable_encodings) = acceptable_encodings {
            req.header(http::header::ACCEPT_ENCODING, acceptable_encodings)
        } else {
            req
        };
        let req = req.body::<Vec<u8>>(vec![]).expect(
            "Could not build a test request with Accept-Coding of value {acceptable_encodings}",
        );
        let req = Box::new(req);

        let res = http::response::Response::builder()
            .url(try_to_url(req.uri()).expect("TODO"))
            .body(vec![])
            .expect("TODO");

        (req, res)
    }

    fn build_test_interpreter_context<'a>(
        req: Box<http::request::Request<Vec<u8>>>,
        res: Option<Box<http::response::Response<Vec<u8>>>>,
        runtime: &'a tokio::runtime::Handle,
    ) -> SimpleProcessorsInterpreterContext<'a> {
        SimpleProcessorsInterpreterContext {
            scopes: Default::default(),
            mode: InterpretMode::Response,
            runtime,
            req,
            res,
        }
    }

    #[test]
    fn test_allow_compress_interpretation() {
        // Should choose identity (because there is no support for compress or brotli).
        let (req, res) = build_test_request_response(Some("compress;q=1,brotli;q=1"));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("Could not get a Tokio runtime for testing");
        let context = build_test_interpreter_context(req, Some(Box::new(res)), runtime.handle());

        let (result, _) = interpret_allow_compress(context).expect("Could not run the AllowCompress interpreter on a request with a valid Accept-Encoding header");

        let interpreted_response = match result.res {
            Some(response) => response,
            None => panic!(),
        };

        assert_eq!(interpreted_response.headers().len(), 1);

        let header = interpreted_response
            .headers()
            .iter()
            .next()
            .expect("Missing expected header in an interpreted response");

        assert_eq!(header.0, http::header::CONTENT_ENCODING);
        assert_eq!(
            header.1.to_str().expect("TODO"),
            ContentCodingTypes::Identity.to_string()
        );

        // Should choose deflate (because of high q).
        let (req, res) = build_test_request_response(Some("identity;q=0,deflate;q=1"));
        let context = build_test_interpreter_context(req, Some(Box::new(res)), runtime.handle());

        let (result, _) = interpret_allow_compress(context).expect("Could not run the AllowCompress interpreter on a request with a valid Accept-Encoding header");

        let interpreted_response = match result.res {
            Some(response) => response,
            None => panic!(),
        };

        assert_eq!(interpreted_response.headers().len(), 1);

        let header = interpreted_response
            .headers()
            .iter()
            .next()
            .expect("Missing expected header in an interpreted response");

        assert_eq!(header.0, http::header::CONTENT_ENCODING);
        assert_eq!(
            header.1.to_str().expect("TODO"),
            ContentCodingTypes::Deflate.to_string()
        );

        // Should choose identity (because of high q).
        let (req, res) = build_test_request_response(Some("identity;q=1,deflate;q=0.5"));
        let context = build_test_interpreter_context(req, Some(Box::new(res)), runtime.handle());

        let (result, _) = interpret_allow_compress(context).expect("Could not run the AllowCompress interpreter on a request with a valid Accept-Encoding header");

        let interpreted_response = match result.res {
            Some(response) => response,
            None => panic!(),
        };

        assert_eq!(interpreted_response.headers().len(), 1);

        let header = interpreted_response
            .headers()
            .iter()
            .next()
            .expect("Missing expected header in an interpreted response");

        assert_eq!(header.0, http::header::CONTENT_ENCODING);
        assert_eq!(
            header.1.to_str().expect("TODO"),
            ContentCodingTypes::Identity.to_string()
        );

        // Should choose gzip (because of high q).
        let (req, res) =
            build_test_request_response(Some("identity;q=0.75,deflate;q=0.5,gzip;q=1.0"));
        let context = build_test_interpreter_context(req, Some(Box::new(res)), runtime.handle());

        let (result, _) = interpret_allow_compress(context).expect("Could not run the AllowCompress interpreter on a request with a valid Accept-Encoding header");

        let interpreted_response = match result.res {
            Some(response) => response,
            None => panic!(),
        };

        assert_eq!(interpreted_response.headers().len(), 1);

        let header = interpreted_response
            .headers()
            .iter()
            .next()
            .expect("Missing expected header in an interpreted response");

        assert_eq!(header.0, http::header::CONTENT_ENCODING);
        assert_eq!(
            header.1.to_str().expect("TODO"),
            ContentCodingTypes::Gzip.to_string()
        );

        // Should choose identity (because empty Accept-Encoding header value implies desire for no coding)
        let (req, res) = build_test_request_response(Some(""));
        let context = build_test_interpreter_context(req, Some(Box::new(res)), runtime.handle());

        let (result, _) = interpret_allow_compress(context).expect("Could not run the AllowCompress interpreter on a request with a valid Accept-Encoding header");

        let interpreted_response = match result.res {
            Some(response) => response,
            None => panic!(),
        };

        assert_eq!(interpreted_response.headers().len(), 1);

        let header = interpreted_response
            .headers()
            .iter()
            .next()
            .expect("Missing expected header in an interpreted response");

        assert_eq!(header.0, http::header::CONTENT_ENCODING);
        assert_eq!(
            header.1.to_str().expect("TODO"),
            ContentCodingTypes::Identity.to_string()
        );

        // Should choose deflate (because missing Accept-Encoding header implies user's willingness to accept any coding)
        let (req, res) = build_test_request_response(None);
        let context = build_test_interpreter_context(req, Some(Box::new(res)), runtime.handle());

        let (result, _) = interpret_allow_compress(context).expect("Could not run the AllowCompress interpreter on a request with a valid Accept-Encoding header");

        let interpreted_response = match result.res {
            Some(response) => response,
            None => panic!(),
        };

        assert_eq!(interpreted_response.headers().len(), 1);

        let header = interpreted_response
            .headers()
            .iter()
            .next()
            .expect("Missing expected header in an interpreted response");

        assert_eq!(header.0, http::header::CONTENT_ENCODING);
        assert_eq!(
            header.1.to_str().expect("TODO"),
            ContentCodingTypes::Deflate.to_string()
        );
    }
}
