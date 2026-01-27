use anyhow::{Context as _, Result};
use nix_bindings_fetchers_sys as raw;
use nix_bindings_util::check_call;
use nix_bindings_util::context::{self, Context};
use std::ptr::NonNull;
use std::sync::LazyLock;

static INIT: LazyLock<Result<()>> = LazyLock::new(|| unsafe {
    check_call!(raw::libfetchers_init(&mut Context::new()))?;
    Ok(())
});

pub fn init() -> Result<()> {
    let x = INIT.as_ref();
    match x {
        Ok(_) => Ok(()),
        Err(e) => {
            // Couldn't just clone the error, so we have to print it here.
            Err(anyhow::format_err!(
                "nix_bindings_fetchers::init error: {}",
                e
            ))
        }
    }
}

pub struct FetchersSettings {
    pub(crate) ptr: NonNull<raw::fetchers_settings>,
}
impl Drop for FetchersSettings {
    fn drop(&mut self) {
        unsafe {
            raw::fetchers_settings_free(self.ptr.as_ptr());
        }
    }
}
impl FetchersSettings {
    pub fn new() -> Result<Self> {
        init()?;

        let mut ctx = Context::new();
        let ptr = unsafe { context::check_call!(raw::fetchers_settings_new(&mut ctx))? };
        Ok(FetchersSettings {
            ptr: NonNull::new(ptr).context("fetchers_settings_new unexpectedly returned null")?,
        })
    }

    pub fn raw_ptr(&self) -> *mut raw::fetchers_settings {
        self.ptr.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetchers_settings_new() {
        let _ = FetchersSettings::new().unwrap();
    }
}
