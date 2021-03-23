
use std::error::Error as StdError;
use std::fmt;

use hyper::http;

#[derive(Debug)]
pub enum Error {
	Hyper(hyper::Error),
	Http(http::Error),
	Request(Box<dyn StdError + Send + Sync>),
}

impl Error {
	pub(crate) fn request<E: Into<Box<dyn StdError + Send + Sync>>>(e: E) -> Error {
		Error::Request(e.into())
	}
}

impl From<hyper::Error> for Error {
	fn from(e: hyper::Error) -> Error {
		Error::Hyper(e)
	}
}

impl From<http::Error> for Error {
	fn from(e: http::Error) -> Error {
		Error::Http(e)
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "TODO")
	}
}

impl StdError for Error {}
