use crate::eval_state::{EvalState, EvalStateError};
use crate::value::Value;
use nix_bindings_expr_sys as raw;
use nix_bindings_util::check_call;
use nix_bindings_util::context::Context;
use nix_bindings_util_sys as raw_util;
use std::error::Error;
use std::ffi::{c_int, c_void, CStr, CString};
use std::ptr::{null, null_mut};

#[cfg(nix_at_least = "2.34.0pre")]
use nix_bindings_util::Error;

/// Metadata for a primop, used with `PrimOp::new`.
pub struct PrimOpMeta<'a, const N: usize> {
    /// Name of the primop. Note that primops do not have to be registered as
    /// builtins. Nonetheless, a name is required for documentation purposes, e.g.
    /// :doc in the repl.
    pub name: &'a CStr,

    /// Documentation for the primop. This is displayed in the repl when using
    /// :doc. The format is markdown.
    pub doc: &'a CStr,

    /// The number of arguments the function takes, as well as names for the
    /// arguments, to be presented in the documentation (if applicable, e.g.
    /// :doc in the repl).
    pub args: [&'a CStr; N],
}

pub struct PrimOp<'a> {
    ptr: *mut raw::PrimOp,
    eval_state: &'a mut EvalState,
}

impl Drop for PrimOp<'_> {
    fn drop(&mut self) {
        unsafe {
            raw::gc_decref(null_mut(), self.ptr as *mut c_void);
        }
    }
}

impl<'a> PrimOp<'a> {
    /// Create a new primop with the given metadata and implementation.
    ///
    /// When `f` returns an `Err`, the error is propagated to the Nix evaluator.
    /// To return a [recoverable error](RecoverableError), include it in the
    /// error chain (e.g. `Err(RecoverableError::new("...").into())`).
    pub fn new<const N: usize>(
        eval_state: &'a mut EvalState,
        meta: PrimOpMeta<N>,
        f: Box<dyn Fn(&mut EvalState, &[Value; N]) -> Result<Value, Box<dyn Error>>>,
    ) -> Result<PrimOp<'a>, EvalStateError> {
        assert!(N != 0);

        let mut args = Vec::new();
        for arg in meta.args {
            args.push(arg.as_ptr());
        }
        args.push(null());

        // Primops weren't meant to be dynamically created, as of writing.
        // This leaks, and so do the primop fields in Nix internally.
        let user_data = {
            // We'll be leaking this Box.
            // TODO: Use the GC with finalizer, if possible.
            let user_data = Box::leak(Box::new(PrimOpContext {
                arity: N,
                function: Box::new(move |eval_state, args| f(eval_state, args.try_into().unwrap())),
                eval_state,
            }));
            user_data as *const PrimOpContext as *mut c_void
        };
        let mut ctx = Context::new();
        let op = unsafe {
            check_call!(raw::alloc_primop(
                &mut ctx,
                FUNCTION_ADAPTER,
                N as c_int,
                meta.name.as_ptr(),
                args.as_mut_ptr(), /* TODO add an extra const to bindings to avoid mut here. */
                meta.doc.as_ptr(),
                user_data
            ))?
        };

        Ok(PrimOp {
            ptr: op,
            eval_state,
        })
    }

    /// Creates a new [`function`](crate::value::ValueType::Function) Nix value implemented by a Rust function.
    ///
    /// This is also known as a "primop" in Nix, short for primitive operation.
    /// Most of the `builtins.*` values are examples of primops, but this function
    /// does not affect `builtins`.
    #[doc(alias = "make_primop")]
    #[doc(alias = "create_function")]
    #[doc(alias = "builtin")]
    pub fn new_value(mut self) -> Result<Value, EvalStateError> {
        self.with_state_and_ptr(|ptr, this| {
            let value = this.new_value_uninitialized()?;
            let mut ctx = Context::new();
            unsafe {
                check_call!(raw::init_primop(&mut ctx, value.raw_ptr(), ptr))?;
            };
            Ok(value)
        })
    }

    pub(crate) fn with_state_and_ptr<F, T>(&mut self, f: F) -> T
    where
        F: Fn(*mut raw::PrimOp, &mut EvalState) -> T,
    {
        f(self.ptr, self.eval_state)
    }
}

/// The user_data for our Nix primops
struct PrimOpContext<'a> {
    arity: usize,
    function: Box<dyn Fn(&mut EvalState, &[Value]) -> Result<Value, Box<dyn Error>>>,
    eval_state: &'a mut EvalState,
}

unsafe extern "C" fn function_adapter(
    user_data: *mut ::std::os::raw::c_void,
    context_out: *mut raw_util::c_context,
    _state: *mut raw::EvalState,
    args: *mut *mut raw::Value,
    ret: *mut raw::Value,
) {
    let primop_info = (user_data as *mut PrimOpContext).as_mut().unwrap();
    let args_raw_slice = unsafe { std::slice::from_raw_parts(args, primop_info.arity) };
    let args_vec: Vec<Value> = args_raw_slice
        .iter()
        .map(|v| Value::new_borrowed(*v))
        .collect();
    let args_slice = args_vec.as_slice();

    let r = primop_info.function.as_ref()(primop_info.eval_state, args_slice);

    match r {
        Ok(v) => unsafe {
            raw::copy_value(context_out, ret, v.raw_ptr());
        },
        Err(e) => unsafe {
            let err_str = e.to_string();
            let err_code = error_code(e);
            let cstr = CString::new(err_str).unwrap_or_else(|_e| {
                CString::new("<rust nix-expr application error message contained null byte>")
                    .unwrap()
            });
            raw_util::set_err_msg(context_out, err_code, cstr.as_ptr());
        },
    }
}

#[cfg_attr(not(nix_at_least = "2.34.0pre"), allow(unused))]
fn error_code(e: Box<dyn Error>) -> raw_util::err {
    #[cfg(nix_at_least = "2.34.0pre")]
    if e.downcast_ref::<Error>()
        .is_some_and(|e| matches!(e, Error::RecoverableError(_)))
    {
        return raw_util::err_NIX_ERR_RECOVERABLE;
    }
    raw_util::err_NIX_ERR_UNKNOWN
}

static FUNCTION_ADAPTER: raw::PrimOpFun = Some(function_adapter);
