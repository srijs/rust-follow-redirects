use std::io;

use http;
use hyper::{Error, Uri};
use hyper::header::Location;

pub trait UriExt {
    fn compute_redirect(&self, location: &Location) -> Result<Uri, Error>;
}

impl UriExt for Uri {
    fn compute_redirect(&self, location: &Location) -> Result<Uri, Error> {
        let new_uri = location.parse::<http::Uri>()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let old_uri = self.as_ref().parse::<http::Uri>()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let old_parts = http::uri::Parts::from(old_uri);
        let mut new_parts = http::uri::Parts::from(new_uri);
        if new_parts.scheme.is_none() {
            new_parts.scheme = old_parts.scheme;
        }
        if new_parts.authority.is_none() {
            new_parts.authority = old_parts.authority;
        }
        let absolute_new_uri = http::Uri::from_parts(new_parts)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(absolute_new_uri.to_string().parse::<Uri>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::{Location, Uri, UriExt};

    #[test]
    fn retains_scheme_and_authority() {
        let base = "http://example.org/foo?x=1".parse::<Uri>().unwrap();
        let location = Location::new("/bar?y=1");
        let new = base.compute_redirect(&location).unwrap();
        assert_eq!("http://example.org/bar?y=1", new.as_ref());
    }

    #[test]
    fn replaces_scheme_and_authority() {
        let base = "http://example.org/foo?x=1".parse::<Uri>().unwrap();
        let location = Location::new("https://example.com/bar?y=1");
        let new = base.compute_redirect(&location).unwrap();
        assert_eq!("https://example.com/bar?y=1", new.as_ref());
    }
}
