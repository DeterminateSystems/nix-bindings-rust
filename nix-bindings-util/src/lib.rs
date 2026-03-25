use std::{ffi::NulError, str::Utf8Error, string::FromUtf8Error};

use nix_bindings_bindgen_raw::{self as raw};
use thiserror::Error;

pub mod context;
pub mod settings;
#[macro_use]
pub mod string_return;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the Nix API returned an error")]
    Nix(#[from] NixError),
    #[error("GC_get_stack_base failed")]
    GcStackBase(#[from] GcError),
    #[error("string was not set by Nix C API")]
    StringInit,
    #[error("failed to lock store cache; this should never happen")]
    StoreCacheLock,
    #[error("nix_libstore_init error")]
    LibstoreInit(#[source] &'static Self),
    #[error("nix_bindings_expr::init error")]
    NixBindingsExprInit(#[source] &'static Self),
    #[error("{0} returned null")]
    NullDerivation(&'static str),
    #[error("{0} unexpectedly returned null")]
    UnexpectedNullPointer(&'static str),

    #[error(transparent)]
    NulError(#[from] NulError),
    #[error(transparent)]
    StrUtf8Error(#[from] Utf8Error),
    #[error(transparent)]
    StringUtf8Error(#[from] FromUtf8Error),
}

/// An error as returned from the Nix API.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum NixError {
    /// An unknown error.
    ///
    /// See [`err_NIX_ERR_UNKNOWN`](raw::err_NIX_ERR_UNKNOWN) for more info.
    #[error("an unknown error occurred: {0}")]
    Unknown(String),
    /// An overflow error.
    ///
    /// See [`err_NIX_ERR_OVERFLOW`](raw::err_NIX_ERR_OVERFLOW) for more info.
    #[error("an overflow error occurred: {0}")]
    Overflow(String),
    /// An key / index access error error.
    ///
    /// See [`err_NIX_ERR_KEY`](raw::err_NIX_ERR_KEY) for more info.
    #[error("a key / index access error occurred: {0}")]
    Key(String),
    /// An generic Nix error.
    ///
    /// See [`err_NIX_ERR_NIX_ERROR`](raw::err_NIX_ERR_NIX_ERROR) for more info.
    #[error("a generic Nix error occurred: {0}")]
    Nix(String),
    /// An unknown error code that is not (yet) handled by this crate.
    #[error("unknown error code {code}: {msg}")]
    UnknownErrorCode { code: i32, msg: String },
}

impl NixError {
    pub fn new(code: i32, msg: String) -> Self {
        match code {
            raw::err_NIX_ERR_NIX_ERROR => Self::Nix(msg),
            raw::err_NIX_ERR_KEY => Self::Key(msg),
            raw::err_NIX_ERR_OVERFLOW => Self::Overflow(msg),
            raw::err_NIX_ERR_UNKNOWN => Self::Unknown(msg),
            code => Self::UnknownErrorCode { code, msg },
        }
    }
}

/// An error as returend
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum GcError {
    #[error("not enough memory")]
    NoMemory,
    #[error("duplicate allocation")] // TODO: ???
    Duplicate,
    #[error("not enough threads")] // TODO: ???
    NoThreads,
    #[error("not implemented")]
    Unimplemented,
    #[error("not found")]
    NotFound,
    /// An unknown error code that is not (yet) handled by this crate.
    #[error("unknown error code {0}")]
    UnknownErrorCode(i32),
}

impl From<i32> for GcError {
    fn from(value: i32) -> Self {
        let Ok(errval) = value.try_into() else {
            return Self::UnknownErrorCode(value);
        };

        match errval {
            raw::GC_NO_MEMORY => Self::NoMemory,
            raw::GC_DUPLICATE => Self::Duplicate,
            raw::GC_NOT_FOUND => Self::NotFound,
            raw::GC_UNIMPLEMENTED => Self::Unimplemented,
            _ => Self::UnknownErrorCode(value),
        }
    }
}
