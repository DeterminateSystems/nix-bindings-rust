use crate::raw_sys as raw;
use nix_bindings_bdwgc_sys as raw_gc;
use std::ffi::{c_char, CStr};
use std::{ffi::NulError, str::Utf8Error, string::FromUtf8Error};

use thiserror::Error;

pub mod context;
pub mod settings;
#[macro_use]
pub mod string_return;
pub mod nix_version;

// Re-export for use in macros
pub use nix_bindings_util_sys as raw_sys;

pub type Result<T> = std::result::Result<T, Error>;

/// Error type for the nix-bindings-util crate.
///
/// This enum wraps errors from several sources:
/// - Nix C API errors ([`NixError`])
/// - Nix garbage collector errors ([`GcError`])
/// - String/derivation initialization tracking
/// - Null pointer errors from the Nix C API
/// - Standard library conversion errors
#[derive(Error, Debug)]
pub enum Error {
    /// An error from the Nix C API.
    ///
    /// Contains detailed error information from the underlying Nix C API.
    /// See [`NixError`] for the specific error kinds.
    #[error("the Nix API returned an error")]
    Nix(#[from] NixError),
    /// The garbage collector failed to get the stack base.
    ///
    /// This is used during thread registration with the Nix garbage collector.
    /// See [`GcError`] for details on the underlying GC errors.
    #[error("GC_get_stack_base failed")]
    GcStackBase(#[from] GcError),
    /// A string callback was called but the result was not initialized.
    ///
    /// This occurs when using `callback_get_result_string` and the Nix C API
    /// does not properly set the result string.
    #[error("string was not set by Nix C API")]
    StringInit,
    /// A derivation callback was called but the result was not initialized.
    ///
    /// This occurs when using `callback_get_result_derivation` and the Nix C API
    /// does not properly set the result derivation.
    #[error("Derivation was not set by Nix C API")]
    DerivationInit,
    /// Failed to acquire the store cache lock.
    ///
    /// This error should never occur in correct usage.
    #[error("failed to lock store cache; this should never happen")]
    StoreCacheLock,
    /// Initialization of `nix_libstore` failed.
    ///
    /// The wrapped error contains the original initialization failure.
    #[error("nix_libstore_init error")]
    LibstoreInit(#[source] &'static Self),
    /// Initialization of `nix_bindings_expr` failed.
    ///
    /// The wrapped error contains the original initialization failure.
    #[error("nix_bindings_expr::init error")]
    NixBindingsExprInit(#[source] &'static Self),
    /// A Nix C API function returned a null pointer for a derivation.
    ///
    /// This indicates that a function expected to return a valid derivation
    /// pointer instead returned null, suggesting the operation failed.
    ///
    /// The inner string is the name of the function that returned null.
    #[error("{0} returned null")]
    NullDerivation(&'static str),
    /// A Nix C API function returned a null pointer where a valid pointer was expected.
    ///
    /// This indicates an unexpected null return value from a function that should
    /// always return a valid pointer on success.
    ///
    /// The inner string is the name of the function that returned null.
    #[error("{0} unexpectedly returned null")]
    UnexpectedNullPointer(&'static str),

    /// A C string contained a null byte.
    #[error(transparent)]
    NulError(#[from] NulError),
    /// A byte slice that needed to be valid UTF-8 was not valid UTF-8.
    #[error(transparent)]
    StrUtf8Error(#[from] Utf8Error),
    /// A string that needed to be valid UTF-8 was not valid UTF-8.
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

/// An error as returned by the Nix garbage collector.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum GcError {
    #[error("not enough memory")]
    NoMemory,
    #[error("duplicate allocation")]
    Duplicate,
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
            raw_gc::GC_NO_MEMORY => Self::NoMemory,
            raw_gc::GC_DUPLICATE => Self::Duplicate,
            raw_gc::GC_NOT_FOUND => Self::NotFound,
            raw_gc::GC_UNIMPLEMENTED => Self::Unimplemented,
            _ => Self::UnknownErrorCode(value),
        }
    }
}

#[doc(alias = "nix_libutil_init")]
pub fn init() -> Result<()> {
    let mut ctx = context::Context::new();
    unsafe {
        check_call!(raw::libutil_init(&mut ctx))?;
    }
    Ok(())
}

#[doc(alias = "nix_version_get")]
pub fn get_version() -> Result<&'static str> {
    let c_str = unsafe {
        let ptr = raw::version_get();
        CStr::from_ptr(ptr as *const c_char)
    };

    Ok(c_str.to_str()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nix_version::parse_version;

    #[test]
    fn init() {
        super::init().unwrap();
    }

    #[test]
    fn version() {
        assert!(parse_version(get_version().unwrap()) > (0, 0, 0));
    }
}
