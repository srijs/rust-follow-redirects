use std::io;

use hyper::header::HeaderValue;
use hyper::http;
use hyper::Uri;

use crate::error::Error;

pub(crate) trait UriExt {
    fn compute_redirect(&self, location: &HeaderValue) -> Result<Uri, Error>;
    fn is_same_host(&self, other: &Uri) -> bool;
}

impl UriExt for Uri {
    fn compute_redirect(&self, location: &HeaderValue) -> Result<Uri, Error> {
        //TODO(stevenroose) fix errors
        let new_uri = location.to_str().unwrap_or("").parse::<Uri>().expect("TODO");
        let old_parts = self.clone().into_parts();
        let mut new_parts = new_uri.into_parts();
        if new_parts.scheme.is_none() {
            new_parts.scheme = old_parts.scheme;
        }
        if new_parts.authority.is_none() {
            new_parts.authority = old_parts.authority;
        }
        let absolute_new_uri = Uri::from_parts(new_parts).expect("TODO");
        Ok(absolute_new_uri)
    }

    fn is_same_host(&self, other: &Uri) -> bool {
        self.host() == other.host() && self.port() == other.port()
    }
}

#[cfg(test)]
mod tests {
    use super::{HeaderValue, Uri, UriExt};

    #[test]
    fn extends_empty_path() {
        let base = "http://example.org".parse::<Uri>().unwrap();
        let location = HeaderValue::from_str("/index.html").unwrap();
        let new = base.compute_redirect(&location).unwrap();
        assert_eq!("http://example.org/index.html", &new.to_string());
    }

    #[test]
    fn retains_scheme_and_authority() {
        let base = "http://example.org/foo?x=1".parse::<Uri>().unwrap();
        let location = HeaderValue::from_str("/bar?y=1").unwrap();
        let new = base.compute_redirect(&location).unwrap();
        assert_eq!("http://example.org/bar?y=1", &new.to_string());
    }

    #[test]
    fn replaces_scheme_and_authority() {
        let base = "http://example.org/foo?x=1".parse::<Uri>().unwrap();
        let location = HeaderValue::from_str("https://example.com/bar?y=1").unwrap();
        let new = base.compute_redirect(&location).unwrap();
        assert_eq!("https://example.com/bar?y=1", &new.to_string());
    }
}
