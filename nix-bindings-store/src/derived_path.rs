use std::ptr::NonNull;

use super::path::StorePath;

use anyhow::Result;
use nix_bindings_bindgen_raw as raw;
use nix_bindings_util::context::Context;
use nix_bindings_util::{check_call, result_string_init};

pub struct DerivedPath {
    raw: NonNull<raw::DerivedPath>,
}
impl DerivedPath {
    pub fn get_store_path(&self) -> Result<StorePath> {
        let mut context = Context::new();
        unsafe {
            let store_path = check_call!(raw::derived_path_get_store_path(
                &mut context,
                self.as_ptr()
            ))?;
            let store_path =
                NonNull::new(store_path).expect("nix_derived_path_get_store_path returned a null pointer");
            Ok(StorePath::new_raw(store_path))
        }
    }

    /// This is a low level function that you shouldn't have to call unless you are developing the Nix bindings.
    ///
    /// Construct a new `DerivedPath` by first cloning the C derived path.
    ///
    /// # Safety
    ///
    /// This does not take ownership of the C derived path, so it should be a borrowed pointer, or you should free it.
    pub unsafe fn new_raw_clone(raw: NonNull<raw::DerivedPath>) -> Self {
        Self::new_raw(
            NonNull::new(raw::derived_path_clone(raw.as_ptr()))
                .or_else(|| panic!("nix_derived_path_clone returned a null pointer"))
                .unwrap(),
        )
    }

    /// This is a low level function that you shouldn't have to call unless you are developing the Nix bindings.
    ///
    /// Takes ownership of a C `nix_derived_path`. It will be freed when the `DerivedPath` is dropped.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided `NonNull<raw::DerivedPath>` is valid and that the ownership
    /// semantics are correctly followed. The `raw` pointer must not be used after being passed to this function.
    pub unsafe fn new_raw(raw: NonNull<raw::DerivedPath>) -> Self {
        DerivedPath { raw }
    }

    /// This is a low level function that you shouldn't have to call unless you are developing the Nix bindings.
    ///
    /// Get a pointer to the underlying Nix C API derived path.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it returns a raw pointer. The caller must ensure that the pointer is not used beyond the lifetime of this `DerivedPath`.
    pub unsafe fn as_ptr(&self) -> *mut raw::DerivedPath {
        self.raw.as_ptr()
    }
}
impl Drop for DerivedPath {
    fn drop(&mut self) {
        unsafe {
            raw::derived_path_free(self.as_ptr());
        }
    }
}
