use assert_json_diff::assert_json_eq_no_panic;
use http::{Request, Uri};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ProtocolTestFailure {
    #[error("missing query param: expected `{expected}`, found {found:?}")]
    MissingQueryParam {
        expected: String,
        found: Vec<String>,
    },
    #[error("forbidden query param present: `{expected}`")]
    ForbiddenQueryParam { expected: String },
    #[error("required query param missing: `{expected}`")]
    RequiredQueryParam { expected: String },

    #[error("invalid header value for key `{key}`: expected `{expected}`, found `{found}`")]
    InvalidHeader {
        key: String,
        expected: String,
        found: String,
    },
    #[error("missing required header: `{expected}`")]
    MissingHeader { expected: String },
    #[error("Header `{forbidden}` was forbidden but found: `{found}`")]
    ForbiddenHeader { forbidden: String, found: String },
    #[error("body did not match. Hint:\n{hint}")]
    BodyDidNotMatch {
        expected: String,
        found: String,
        hint: String,
    },
    #[error("Expected body to be valid {expected} but instead: {found}")]
    InvalidBodyFormat { expected: String, found: String },
}

/// Check that the protocol test succeeded & print the pretty error
/// if it did not
///
/// The primary motivation is making multiline debug output
/// readable & using the cleaner Display implementation
#[track_caller]
pub fn assert_ok(inp: Result<(), ProtocolTestFailure>) {
    match inp {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            panic!("Protocol test failed");
        }
    }
}

#[derive(Eq, PartialEq, Hash)]
struct QueryParam<'a> {
    key: &'a str,
    value: Option<&'a str>,
}

impl<'a> QueryParam<'a> {
    fn parse(s: &'a str) -> Self {
        let mut parsed = s.split('=');
        QueryParam {
            key: parsed.next().unwrap(),
            value: parsed.next(),
        }
    }
}

fn extract_params(uri: &Uri) -> HashSet<&str> {
    uri.query().unwrap_or_default().split('&').collect()
}

pub fn validate_query_string<B>(
    request: &Request<B>,
    expected_params: &[&str],
) -> Result<(), ProtocolTestFailure> {
    let actual_params = extract_params(request.uri());
    for param in expected_params {
        if !actual_params.contains(param) {
            return Err(ProtocolTestFailure::MissingQueryParam {
                expected: param.to_string(),
                found: actual_params.iter().map(|s| s.to_string()).collect(),
            });
        }
    }
    Ok(())
}

pub fn forbid_query_params<B>(
    request: &Request<B>,
    forbid_keys: &[&str],
) -> Result<(), ProtocolTestFailure> {
    let actual_keys: HashSet<&str> = extract_params(request.uri())
        .iter()
        .map(|param| QueryParam::parse(param).key)
        .collect();
    for key in forbid_keys {
        if actual_keys.contains(*key) {
            return Err(ProtocolTestFailure::ForbiddenQueryParam {
                expected: key.to_string(),
            });
        }
    }
    Ok(())
}

pub fn require_query_params<B>(
    request: &Request<B>,
    require_keys: &[&str],
) -> Result<(), ProtocolTestFailure> {
    let actual_keys: HashSet<&str> = extract_params(request.uri())
        .iter()
        .map(|param| QueryParam::parse(param).key)
        .collect();
    for key in require_keys {
        if !actual_keys.contains(*key) {
            return Err(ProtocolTestFailure::RequiredQueryParam {
                expected: key.to_string(),
            });
        }
    }
    Ok(())
}

pub fn validate_headers<B>(
    request: &Request<B>,
    expected_headers: &[(&str, &str)],
) -> Result<(), ProtocolTestFailure> {
    for (key, expected_value) in expected_headers {
        match normalized_header(request, key) {
            None => {
                return Err(ProtocolTestFailure::MissingHeader {
                    expected: key.to_string(),
                })
            }
            Some(actual_value) if actual_value != *expected_value => {
                return Err(ProtocolTestFailure::InvalidHeader {
                    key: key.to_string(),
                    expected: expected_value.to_string(),
                    found: actual_value,
                })
            }
            _ => (),
        }
    }
    Ok(())
}

fn normalized_header<B>(request: &Request<B>, key: &str) -> Option<String> {
    if !request.headers().contains_key(key) {
        None
    } else {
        Some(
            request
                .headers()
                .get_all(key)
                .iter()
                .map(|hv| hv.to_str().unwrap())
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}

pub fn forbid_headers<B>(
    request: &Request<B>,
    forbidden_headers: &[&str],
) -> Result<(), ProtocolTestFailure> {
    for key in forbidden_headers {
        // Protocol tests store header lists as comma-delimited
        if let Some(value) = normalized_header(request, *key) {
            return Err(ProtocolTestFailure::ForbiddenHeader {
                forbidden: key.to_string(),
                found: format!("{}: {}", key, value),
            });
        }
    }
    Ok(())
}

pub fn require_headers<B>(
    request: &Request<B>,
    required_headers: &[&str],
) -> Result<(), ProtocolTestFailure> {
    for key in required_headers {
        // Protocol tests store header lists as comma-delimited
        if normalized_header(request, *key).is_none() {
            return Err(ProtocolTestFailure::MissingHeader {
                expected: key.to_string(),
            });
        }
    }
    Ok(())
}

pub enum MediaType {
    /// Json media types are deserialized and compared
    Json,
    /// Other media types are compared literally
    // TODO: XML, etc.
    Other(String),
}

impl<T: AsRef<str>> From<T> for MediaType {
    fn from(inp: T) -> Self {
        match inp.as_ref() {
            "application/json" => MediaType::Json,
            other => MediaType::Other(other.to_string()),
        }
    }
}

pub fn validate_body<T: AsRef<[u8]>>(
    actual_body: T,
    expected_body: &str,
    media_type: MediaType,
) -> Result<(), ProtocolTestFailure> {
    let body_str = std::str::from_utf8(actual_body.as_ref());
    match (media_type, body_str) {
        (MediaType::Json, Ok(actual_body)) => validate_json_body(actual_body, expected_body),
        (MediaType::Json, Err(_)) => Err(ProtocolTestFailure::InvalidBodyFormat {
            expected: "json".to_owned(),
            found: "input was not valid UTF-8".to_owned(),
        }),
        (MediaType::Other(media_type), Ok(actual_body)) => {
            if actual_body != expected_body {
                Err(ProtocolTestFailure::BodyDidNotMatch {
                    expected: expected_body.to_string(),
                    found: actual_body.to_string(),
                    hint: format!("media type: {}", media_type),
                })
            } else {
                Ok(())
            }
        }
        // It's not clear from the Smithy spec exactly how a binary / base64 encoded body is supposed
        // to work. Defer implementation for now until an actual test exists.
        (MediaType::Other(_), Err(_)) => {
            unimplemented!("binary/non-utf8 formats not yet supported")
        }
    }
}

fn validate_json_body(actual: &str, expected: &str) -> Result<(), ProtocolTestFailure> {
    let actual_json: serde_json::Value =
        serde_json::from_str(actual).map_err(|e| ProtocolTestFailure::InvalidBodyFormat {
            expected: "json".to_owned(),
            found: e.to_string(),
        })?;
    let expected_json: serde_json::Value =
        serde_json::from_str(expected).expect("expected value must be valid JSON");
    match assert_json_eq_no_panic(&actual_json, &expected_json) {
        Ok(()) => Ok(()),
        Err(message) => Err(ProtocolTestFailure::BodyDidNotMatch {
            expected: expected.to_string(),
            found: actual.to_string(),
            hint: message,
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        forbid_headers, forbid_query_params, require_headers, require_query_params, validate_body,
        validate_headers, validate_query_string, MediaType, ProtocolTestFailure,
    };
    use http::Request;

    #[test]
    fn test_validate_empty_query_string() {
        let request = Request::builder().uri("/foo").body(()).unwrap();
        validate_query_string(&request, &[]).expect("no required params should pass");
        validate_query_string(&request, &["a"])
            .err()
            .expect("no params provided");
    }

    #[test]
    fn test_validate_query_string() {
        let request = Request::builder()
            .uri("/foo?a=b&c&d=efg&hello=a%20b")
            .body(())
            .unwrap();
        validate_query_string(&request, &["a=b"]).expect("a=b is in the query string");
        validate_query_string(&request, &["c", "a=b"])
            .expect("both params are in the query string");
        validate_query_string(&request, &["a=b", "c", "d=efg", "hello=a%20b"])
            .expect("all params are in the query string");
        validate_query_string(&request, &[]).expect("no required params should pass");

        validate_query_string(&request, &["a"]).expect_err("no parameter should match");
        validate_query_string(&request, &["a=bc"]).expect_err("no parameter should match");
        validate_query_string(&request, &["a=bc"]).expect_err("no parameter should match");
        validate_query_string(&request, &["hell=a%20"]).expect_err("no parameter should match");
    }

    #[test]
    fn test_forbid_query_param() {
        let request = Request::builder()
            .uri("/foo?a=b&c&d=efg&hello=a%20b")
            .body(())
            .unwrap();
        forbid_query_params(&request, &["a"]).expect_err("a is a query param");
        forbid_query_params(&request, &["not_included"]).expect("query param not included");
        forbid_query_params(&request, &["a=b"]).expect("should be matching against keys");
        forbid_query_params(&request, &["c"]).expect_err("c is a query param");
    }

    #[test]
    fn test_require_query_param() {
        let request = Request::builder()
            .uri("/foo?a=b&c&d=efg&hello=a%20b")
            .body(())
            .unwrap();
        require_query_params(&request, &["a"]).expect("a is a query param");
        require_query_params(&request, &["not_included"]).expect_err("query param not included");
        require_query_params(&request, &["a=b"]).expect_err("should be matching against keys");
        require_query_params(&request, &["c"]).expect("c is a query param");
    }

    #[test]
    fn test_validate_headers() {
        let request = Request::builder()
            .uri("/")
            .header("X-Foo", "foo")
            .header("X-Foo-List", "foo")
            .header("X-Foo-List", "bar")
            .header("X-Inline", "inline, other")
            .body(())
            .unwrap();

        validate_headers(&request, &[("X-Foo", "foo")]).expect("header present");
        validate_headers(&request, &[("X-Foo", "Foo")]).expect_err("case sensitive");
        validate_headers(&request, &[("x-foo-list", "foo, bar")]).expect("list concat");
        validate_headers(&request, &[("X-Foo-List", "foo")])
            .expect_err("all list members must be specified");
        validate_headers(&request, &[("X-Inline", "inline, other")])
            .expect("inline header lists also work");
        assert_eq!(
            validate_headers(&request, &[("missing", "value")]),
            Err(ProtocolTestFailure::MissingHeader {
                expected: "missing".to_owned()
            })
        );
    }

    #[test]
    fn test_forbidden_headers() {
        let request = Request::builder()
            .uri("/")
            .header("X-Foo", "foo")
            .body(())
            .unwrap();
        assert_eq!(
            forbid_headers(&request, &["X-Foo"]).expect_err("should be error"),
            ProtocolTestFailure::ForbiddenHeader {
                forbidden: "X-Foo".to_string(),
                found: "X-Foo: foo".to_string()
            }
        );
        forbid_headers(&request, &["X-Bar"]).expect("header not present");
    }

    #[test]
    fn test_required_headers() {
        let request = Request::builder()
            .uri("/")
            .header("X-Foo", "foo")
            .body(())
            .unwrap();
        require_headers(&request, &["X-Foo"]).expect("header present");
        require_headers(&request, &["X-Bar"]).expect_err("header not present");
    }

    #[test]
    fn test_validate_json_body() {
        let expected = r#"{"abc": 5 }"#;
        let actual = r#"   {"abc":   5 }"#;
        validate_body(&actual, expected, MediaType::Json).expect("inputs matched as JSON");

        let expected = r#"{"abc": 5 }"#;
        let actual = r#"   {"abc":   6 }"#;
        validate_body(&actual, expected, MediaType::Json).expect_err("bodies do not match");
    }

    #[test]
    fn test_validate_non_json_body() {
        let expected = r#"asdf"#;
        let actual = r#"asdf "#;
        validate_body(&actual, expected, MediaType::from("something/else"))
            .expect_err("bodies do not match");

        validate_body(&expected, expected, MediaType::from("something/else"))
            .expect("inputs matched exactly")
    }
}