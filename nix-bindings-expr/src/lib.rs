#[cfg(not(feature = "detnix"))]
pub mod eval_state;
#[cfg(feature = "detnix")]
#[doc(hidden)]
pub mod eval_state_detnix;
#[cfg(feature = "detnix")]
pub use eval_state_detnix as eval_state;

#[cfg(not(feature = "detnix"))]
pub mod primop;
#[cfg(feature = "detnix")]
#[doc(hidden)]
pub mod primop_detnix;
#[cfg(feature = "detnix")]
pub use primop_detnix as primop;

pub mod value;
