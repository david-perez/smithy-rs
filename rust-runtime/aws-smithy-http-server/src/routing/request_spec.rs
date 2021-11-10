use std::collections::HashMap;

use http::Request;
use regex::Regex;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum PathSegment {
    Literal(String),
    Label,
    Greedy,
}

#[derive(Debug, Clone)]
pub enum QuerySegment {
    Key(String),
    KeyValue(String, String),
}

#[derive(Debug, Clone)]
pub enum HostPrefixSegment {
    Literal(String),
    Label,
}

// TODO The struct does not prevent us from adding multiple greedy labels, or not putting greedy
// labels last.
#[derive(Debug, Clone, Default)]
pub struct PathSpec(pub Vec<PathSegment>);

pub type QuerySpec = Vec<QuerySegment>;

#[derive(Debug, Clone, Default)]
pub struct PathAndQuerySpec {
    pub path_segments: PathSpec,
    pub query_segments: QuerySpec,
}

#[derive(Debug, Clone)]
pub struct UriSpec {
    #[builder(default)]
    pub host_prefix: Option<Vec<HostPrefixSegment>>,
    #[builder(default)]
    pub path_and_query: PathAndQuerySpec,
}

#[derive(Debug, Clone)]
pub struct RequestSpec {
    method: http::Method,
    uri_spec: UriSpec,
    uri_path_regex: Regex,
}

#[derive(Debug)]
pub enum Match {
    /// The request matches the URI pattern spec.
    Yes,
    /// The request matches the URI pattern spec, but the wrong HTTP method was used. `405 Method
    /// Not Allowed` should be returned in the response.
    MethodNotAllowed,
    /// The request does not match the URI pattern spec. `404 Not Found` should be returned in the
    /// response.
    No,
}

impl From<&PathSpec> for Regex {
    fn from(uri_path_spec: &PathSpec) -> Self {
        let sep = "/+";
        let re = uri_path_spec
            .0
            .iter()
            .map(|segment_spec| match segment_spec {
                PathSegment::Literal(literal) => literal,
                // TODO Should we allow empty segments as valid and pass `""` as the captured
                // label?
                // TODO URL spec says it should be ASCII but this regex accepts UTF-8:
                // https://url.spec.whatwg.org/#url-representation
                PathSegment::Label => "[^/]+",
                PathSegment::Greedy => ".*",
            })
            .fold(String::new(), |a, b| a + sep + b);

        Regex::new(&format!("{}$", re)).unwrap()
    }
}

impl RequestSpec {
    pub fn new(method: http::Method, uri_spec: UriSpec) -> Self {
        let uri_path_regex = (&uri_spec.path_and_query.path_segments).into();
        RequestSpec {
            method,
            uri_spec,
            uri_path_regex,
        }
    }

    pub(super) fn matches<B>(&self, req: &Request<B>) -> Match {
        if let Some(_host_prefix) = &self.uri_spec.host_prefix {
            todo!("Look at host prefix");
        }

        if !self.uri_path_regex.is_match(req.uri().path()) {
            return Match::No;
        }

        if self.uri_spec.path_and_query.query_segments.is_empty() {
            if self.method == req.method() {
                return Match::Yes;
            } else {
                return Match::MethodNotAllowed;
            }
        }

        match req.uri().query() {
            Some(query) => {
                let res = serde_urlencoded::from_str::<HashMap<&str, &str>>(query);

                match res {
                    Err(_) => Match::No,
                    Ok(query_map) => {
                        for query_segment in self.uri_spec.path_and_query.query_segments.iter() {
                            match query_segment {
                                QuerySegment::Key(key) => {
                                    if !query_map.contains_key(key.as_str()) {
                                        return Match::No;
                                    }
                                }
                                QuerySegment::KeyValue(key, expected_value) => {
                                    match query_map.get(key.as_str()) {
                                        None => return Match::No,
                                        Some(found_value) => {
                                            if found_value != expected_value {
                                                return Match::No;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if self.method == req.method() {
                            Match::Yes
                        } else {
                            Match::MethodNotAllowed
                        }
                    }
                }
            }
            None => Match::No,
        }
    }

    pub fn always_get() -> Self {
        RequestSpec {
            method: http::Method::GET,
            uri_spec: UriSpecBuilder::default().build().unwrap(),
            uri_path_regex: Regex::new(".*").unwrap(),
        }
    }
}

/// Errors that can occur when parsing a path and query spec from a string.  See the [Smithy spec
/// for the `uri` HTTP binding
/// trait](https://awslabs.github.io/smithy/1.0/spec/core/http-traits.html#uri) for where most of
/// these variants come from.
#[derive(Debug, Error, PartialEq)]
pub enum PathAndQuerySpecParseError {
    #[error("uri pattern MUST start with \"/\"")]
    DoesNotStartWithForwardSlash,
    #[error("uri pattern MUST NOT end with \"?\"")]
    EndsWithQuestionMark,
    #[error("uri pattern MUST NOT contain empty path segments (a trailing forward slash is considered to create an empty path segment)")]
    ContainsEmptyPathSegment,
    #[error("uri pattern MUST NOT contain empty path segments")]
    ContainsFragment,
    #[error("uri pattern MUST NOT contain empty path segments")]
    ContainsDotSegment,

    // These two are perhaps too strict. In theory, the Smithy spec does not preclude matching
    // literals starting or ending with `{`, `}`, respectively. It also does not specify how to
    // match literals containing `{`, `}`, nor if it is even allowed; note that our implementation
    // allows for these, without even having to escape them.
    #[error("uri pattern contains path segment `{0}` ending with `}}`, but the label's opening `{{` was not found")]
    UnopenedLabel(String),
    #[error("uri pattern contains path segment `{0}` starting with `{{`, but the label's closing `}}` was not found")]
    UnclosedLabel(String),

    #[error("invalid query string spec: {0}")]
    InvalidQuerySpec(#[from] serde_urlencoded::de::Error),
}

#[cfg(test)]
impl PathAndQuerySpec {
    // Private function for use in tests, to help in building `PathAndQuerySpec`s.
    fn parse(s: &str) -> Result<Self, PathAndQuerySpecParseError> {
        let first_char = s
            .chars()
            .next()
            .ok_or(PathAndQuerySpecParseError::DoesNotStartWithForwardSlash)?;
        if first_char != '/' {
            return Err(PathAndQuerySpecParseError::DoesNotStartWithForwardSlash);
        }

        let last_char = s
            .chars()
            .last()
            .expect("we checked above that `s` is not empty");
        if last_char == '?' {
            return Err(PathAndQuerySpecParseError::EndsWithQuestionMark);
        }

        if s.contains("#") {
            return Err(PathAndQuerySpecParseError::ContainsFragment);
        }

        if s.contains(".") {
            return Err(PathAndQuerySpecParseError::ContainsDotSegment);
        }

        let mut iter = s.split('?');
        let path = iter
            .next()
            .expect("`split()` always returns an iterator with at least one element");

        let path_spec = if path == "/" {
            PathSpec(Vec::new())
        } else {
            PathSpec(
                // TODO Validate only one greedy label, and no labels after it.
                // TODO Validate no empty labels.
                path.split('/')
                    // Skip the first element, since it will always be empty because of the compulsory leading forward
                    // slash in the pattern, which we already checked exists.
                    .skip(1)
                    .map(|path_segment| {
                        let first_char = path_segment
                            .chars()
                            .next()
                            .ok_or(PathAndQuerySpecParseError::ContainsEmptyPathSegment)?;

                        let mut last_two_chars = path_segment.chars().rev().take(2);
                        let last_char = last_two_chars
                            .next()
                            .expect("we checked above that `path_segment` is not empty");
                        let penultimate_char_opt = last_two_chars.next();

                        match (first_char, penultimate_char_opt, last_char) {
                            ('{', Some('+'), '}') => Ok(PathSegment::Greedy),
                            ('{', _, '}') => Ok(PathSegment::Label),
                            ('{', _, _c) => Err(PathAndQuerySpecParseError::UnclosedLabel(
                                String::from(path_segment),
                            )),
                            (_c, _, '}') => Err(PathAndQuerySpecParseError::UnopenedLabel(
                                String::from(path_segment),
                            )),
                            _ => Ok(PathSegment::Literal(String::from(path_segment))),
                        }
                    })
                    .collect::<Result<Vec<PathSegment>, PathAndQuerySpecParseError>>()?,
            )
        };

        let query_opt = iter.next();
        match query_opt {
            None => Ok(PathAndQuerySpec {
                path_segments: path_spec,
                query_segments: Vec::new(),
            }),
            Some(query_string) => {
                // TODO Interestingly, `Vec<&str, &str>` does not work, which would be better. Why?
                let query_spec = serde_urlencoded::from_str::<HashMap<&str, &str>>(query_string)?
                    .into_iter()
                    .map(|(k, v)| {
                        if v.is_empty() {
                            QuerySegment::Key(String::from(k))
                        } else {
                            QuerySegment::KeyValue(String::from(k), String::from(v))
                        }
                    })
                    .collect();

                Ok(PathAndQuerySpec {
                    path_segments: path_spec,
                    query_segments: query_spec,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_valid_uri_patterns() {
        let patterns = [
            // Literals.
            "/",
            "/path",
            "/小路/💙/طريق",
            // Query strings.
            "/resource?key=value",
            "/resource?key",
            // Labels.
            "/{label1}/{label2}",
            "/{greedy+}",
            "/{greedy+}/suffix",
            // Complex.
            "/path/{label}/literal",
            "/{小路}/path/{greedy+}",
            "/path/{label}/{greedy+}/suffix",
            "/{label}/path/{greedy+}/suffix?key=小路",
        ];

        // TODO We not only need to test that these are valid, but also that they parse into the
        // expected `PathAndQuery` struct.
        for pattern in patterns {
            PathAndQuerySpec::parse(pattern).expect(&format!(
                "pattern `{}` was expected to be valid but it failed to parse into a `PathAndQuerySpec`",
                String::from(pattern)
            ));
        }
    }

    #[tokio::test]
    async fn parse_invalid_uri_patterns() {
        let patterns = [
            (
                "path",
                PathAndQuerySpecParseError::DoesNotStartWithForwardSlash,
            ),
            ("//", PathAndQuerySpecParseError::ContainsEmptyPathSegment),
            (
                "/my/path?",
                PathAndQuerySpecParseError::EndsWithQuestionMark,
            ),
            (
                "/path#fragment",
                PathAndQuerySpecParseError::ContainsFragment,
            ),
            ("/pa.th..to", PathAndQuerySpecParseError::ContainsDotSegment),
            (
                "/{label",
                PathAndQuerySpecParseError::UnclosedLabel(String::from("{label")),
            ),
            (
                "/label}",
                PathAndQuerySpecParseError::UnopenedLabel(String::from("label}")),
            ),
        ];

        for (pattern, expected_error) in patterns {
            assert_eq!(
                expected_error,
                PathAndQuerySpec::parse(pattern).unwrap_err()
            );
        }
    }

    #[tokio::test]
    async fn test_always_get() {
        let request_spec = RequestSpec::always_get();
        let request = Request::builder()
            .method("GET")
            .uri("https://www.rust-lang.org/")
            .body(())
            .unwrap();
        request_spec.matches(&request);
    }
}
