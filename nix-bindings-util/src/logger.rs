//! Custom Nix logger.
//!
//! Replace Nix's global logger with a Rust callback-based one that
//! receives log messages, activities, and string-valued results from
//! anywhere in libnixutil, libnixstore, and libnixexpr.
//!
//! See the [`Logger`] trait and [`set_logger`].

use crate::raw_sys as raw;
use crate::{check_call, context, Result};
use std::ffi::{c_char, c_void, CStr};
use std::sync::Mutex;

/// Verbosity level of a log message.
///
/// Mirrors the C `nix_verbosity` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Verbosity {
    Error,
    Warn,
    Notice,
    Info,
    Talkative,
    Chatty,
    Debug,
    Vomit,
    /// A verbosity level not (yet) modelled by this crate.
    Unknown(u32),
}

impl Verbosity {
    fn from_raw(v: raw::verbosity) -> Self {
        match v {
            raw::verbosity_NIX_LVL_ERROR => Verbosity::Error,
            raw::verbosity_NIX_LVL_WARN => Verbosity::Warn,
            raw::verbosity_NIX_LVL_NOTICE => Verbosity::Notice,
            raw::verbosity_NIX_LVL_INFO => Verbosity::Info,
            raw::verbosity_NIX_LVL_TALKATIVE => Verbosity::Talkative,
            raw::verbosity_NIX_LVL_CHATTY => Verbosity::Chatty,
            raw::verbosity_NIX_LVL_DEBUG => Verbosity::Debug,
            raw::verbosity_NIX_LVL_VOMIT => Verbosity::Vomit,
            other => Verbosity::Unknown(other),
        }
    }
}

/// Identifier for a logger activity. `0` is the implicit root.
pub type ActivityId = u64;

/// Type of an activity.
///
/// Mirrors the C `nix_activity_type` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ActivityType {
    Unknown,
    CopyPath,
    FileTransfer,
    Realise,
    CopyPaths,
    Builds,
    Build,
    OptimiseStore,
    VerifyPaths,
    Substitute,
    QueryPathInfo,
    PostBuildHook,
    BuildWaiting,
    FetchTree,
    /// An activity type not (yet) modelled by this crate.
    Other(u32),
}

impl ActivityType {
    fn from_raw(t: raw::activity_type) -> Self {
        match t {
            raw::activity_type_NIX_ACTIVITY_TYPE_UNKNOWN => ActivityType::Unknown,
            raw::activity_type_NIX_ACTIVITY_TYPE_COPY_PATH => ActivityType::CopyPath,
            raw::activity_type_NIX_ACTIVITY_TYPE_FILE_TRANSFER => ActivityType::FileTransfer,
            raw::activity_type_NIX_ACTIVITY_TYPE_REALISE => ActivityType::Realise,
            raw::activity_type_NIX_ACTIVITY_TYPE_COPY_PATHS => ActivityType::CopyPaths,
            raw::activity_type_NIX_ACTIVITY_TYPE_BUILDS => ActivityType::Builds,
            raw::activity_type_NIX_ACTIVITY_TYPE_BUILD => ActivityType::Build,
            raw::activity_type_NIX_ACTIVITY_TYPE_OPTIMISE_STORE => ActivityType::OptimiseStore,
            raw::activity_type_NIX_ACTIVITY_TYPE_VERIFY_PATHS => ActivityType::VerifyPaths,
            raw::activity_type_NIX_ACTIVITY_TYPE_SUBSTITUTE => ActivityType::Substitute,
            raw::activity_type_NIX_ACTIVITY_TYPE_QUERY_PATH_INFO => ActivityType::QueryPathInfo,
            raw::activity_type_NIX_ACTIVITY_TYPE_POST_BUILD_HOOK => ActivityType::PostBuildHook,
            raw::activity_type_NIX_ACTIVITY_TYPE_BUILD_WAITING => ActivityType::BuildWaiting,
            raw::activity_type_NIX_ACTIVITY_TYPE_FETCH_TREE => ActivityType::FetchTree,
            other => ActivityType::Other(other),
        }
    }
}

/// Type of a result reported by an activity.
///
/// Only string-valued result types are delivered to
/// [`Logger::result_string`]; other variants exist for forward
/// compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResultType {
    FileLinked,
    BuildLogLine,
    UntrustedPath,
    CorruptedPath,
    SetPhase,
    Progress,
    SetExpected,
    PostBuildLogLine,
    FetchStatus,
    HashMismatch,
    BuildResult,
    /// A result type not (yet) modelled by this crate.
    Other(u32),
}

impl ResultType {
    fn from_raw(t: raw::result_type) -> Self {
        match t {
            raw::result_type_NIX_RESULT_TYPE_FILE_LINKED => ResultType::FileLinked,
            raw::result_type_NIX_RESULT_TYPE_BUILD_LOG_LINE => ResultType::BuildLogLine,
            raw::result_type_NIX_RESULT_TYPE_UNTRUSTED_PATH => ResultType::UntrustedPath,
            raw::result_type_NIX_RESULT_TYPE_CORRUPTED_PATH => ResultType::CorruptedPath,
            raw::result_type_NIX_RESULT_TYPE_SET_PHASE => ResultType::SetPhase,
            raw::result_type_NIX_RESULT_TYPE_PROGRESS => ResultType::Progress,
            raw::result_type_NIX_RESULT_TYPE_SET_EXPECTED => ResultType::SetExpected,
            raw::result_type_NIX_RESULT_TYPE_POST_BUILD_LOG_LINE => ResultType::PostBuildLogLine,
            raw::result_type_NIX_RESULT_TYPE_FETCH_STATUS => ResultType::FetchStatus,
            raw::result_type_NIX_RESULT_TYPE_HASH_MISMATCH => ResultType::HashMismatch,
            raw::result_type_NIX_RESULT_TYPE_BUILD_RESULT => ResultType::BuildResult,
            other => ResultType::Other(other),
        }
    }
}

/// A logger that receives messages from Nix.
///
/// Implementations must be [`Send`] and [`Sync`] because Nix may
/// invoke the callbacks from arbitrary threads (e.g. during parallel
/// builds).
///
/// All methods have empty default implementations, so an
/// implementation only needs to override the events it cares about.
///
/// # Panics
///
/// Callback methods must not unwind across the FFI boundary.
/// Panics from any of these methods are caught and silently
/// discarded; if your implementation may panic, prefer to handle
/// it yourself.
pub trait Logger: Send + Sync {
    /// An ordinary log message.
    ///
    /// Receives `builtins.trace` output, formatted warnings/errors,
    /// and messages produced through the C++ `printError` /
    /// `printInfo` / `debug` macros.
    ///
    /// `msg` is decoded with [`String::from_utf8_lossy`] to keep the
    /// signature ergonomic.
    fn log(&self, _level: Verbosity, _msg: &str) {}

    /// An activity (build, substitution, ...) has started.
    ///
    /// `parent_id` is `0` for top-level activities.
    fn start_activity(
        &self,
        _activity_id: ActivityId,
        _level: Verbosity,
        _type_: ActivityType,
        _description: &str,
        _parent_id: ActivityId,
    ) {
    }

    /// An activity has stopped.
    fn stop_activity(&self, _activity_id: ActivityId) {}

    /// An activity reported a string-valued result.
    ///
    /// Result types that carry non-string fields (such as
    /// [`ResultType::Progress`]) are not delivered through this
    /// method.
    fn result_string(&self, _activity_id: ActivityId, _type_: ResultType, _msg: &str) {}
}

/// Erased trait object passed across the FFI boundary as `userdata`.
type LoggerObj = Box<dyn Logger + Send + Sync>;

/// Serializes calls to [`set_logger`] from the Rust side.
///
/// Note this does *not* protect against C++ code (or other clients of
/// the C API) replacing the global logger concurrently — it only
/// avoids racing with ourselves.
static SET_LOGGER_MUTEX: Mutex<()> = Mutex::new(());

unsafe extern "C" fn thunk_log(userdata: *mut c_void, level: raw::verbosity, msg: *const c_char) {
    assert!(!userdata.is_null());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let logger = &*(userdata as *const LoggerObj);
        let msg_str = if msg.is_null() {
            std::borrow::Cow::Borrowed("")
        } else {
            CStr::from_ptr(msg).to_string_lossy()
        };
        logger.log(Verbosity::from_raw(level), msg_str.as_ref());
    }));
}

unsafe extern "C" fn thunk_start_activity(
    userdata: *mut c_void,
    activity_id: raw::activity_id,
    level: raw::verbosity,
    type_: raw::activity_type,
    s: *const c_char,
    parent_id: raw::activity_id,
) {
    assert!(!userdata.is_null());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let logger = &*(userdata as *const LoggerObj);
        let s_str = if s.is_null() {
            std::borrow::Cow::Borrowed("")
        } else {
            CStr::from_ptr(s).to_string_lossy()
        };
        logger.start_activity(
            activity_id,
            Verbosity::from_raw(level),
            ActivityType::from_raw(type_),
            s_str.as_ref(),
            parent_id,
        );
    }));
}

unsafe extern "C" fn thunk_stop_activity(userdata: *mut c_void, activity_id: raw::activity_id) {
    assert!(!userdata.is_null());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let logger = &*(userdata as *const LoggerObj);
        logger.stop_activity(activity_id);
    }));
}

unsafe extern "C" fn thunk_result_string(
    userdata: *mut c_void,
    activity_id: raw::activity_id,
    type_: raw::result_type,
    msg: *const c_char,
) {
    assert!(!userdata.is_null());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let logger = &*(userdata as *const LoggerObj);
        let msg_str = if msg.is_null() {
            std::borrow::Cow::Borrowed("")
        } else {
            CStr::from_ptr(msg).to_string_lossy()
        };
        logger.result_string(activity_id, ResultType::from_raw(type_), msg_str.as_ref());
    }));
}

unsafe extern "C" fn thunk_destroy(userdata: *mut c_void) {
    if userdata.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(Box::from_raw(userdata as *mut LoggerObj));
    }));
}

static VTABLE: raw::logger = raw::logger {
    log: Some(thunk_log),
    start_activity: Some(thunk_start_activity),
    stop_activity: Some(thunk_stop_activity),
    result_string: Some(thunk_result_string),
    destroy: Some(thunk_destroy),
};

/// Replace Nix's global logger with `logger`.
///
/// `logger` is moved into a heap allocation owned by Nix; it is
/// dropped when this function is called again (replacing the logger)
/// or at process shutdown.
///
/// # Thread safety
///
/// This function serializes concurrent Rust callers via an internal
/// mutex. It does **not** protect against C++ code mutating
/// `nix::logger` directly. Like [`crate::settings::set`], prefer to
/// install a logger during single-threaded initialization.
#[doc(alias = "nix_set_logger")]
pub fn set_logger<L: Logger + 'static>(logger: L) -> Result<()> {
    let _guard = SET_LOGGER_MUTEX.lock().unwrap();

    let boxed: Box<LoggerObj> = Box::new(Box::new(logger));
    let userdata = Box::into_raw(boxed) as *mut c_void;

    let mut ctx = context::Context::new();
    let res = unsafe { check_call!(raw::set_logger(&mut ctx, &VTABLE, userdata)) };

    if let Err(e) = res {
        // The C side did not accept the logger, so it will not invoke
        // the destroy callback. Reclaim the box ourselves to avoid
        // leaking the user-supplied logger.
        unsafe {
            drop(Box::from_raw(userdata as *mut LoggerObj));
        }
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    #[ctor::ctor]
    fn setup() {
        crate::init().unwrap();
    }

    /// Logger that records when it's dropped, so tests can observe the
    /// destroy callback firing on replacement.
    struct DropFlag {
        dropped: Arc<AtomicBool>,
        log_count: Arc<AtomicUsize>,
    }

    impl Logger for DropFlag {
        fn log(&self, _level: Verbosity, _msg: &str) {
            self.log_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn set_and_replace_calls_destroy() {
        let dropped = Arc::new(AtomicBool::new(false));
        let log_count = Arc::new(AtomicUsize::new(0));

        set_logger(DropFlag {
            dropped: dropped.clone(),
            log_count: log_count.clone(),
        })
        .unwrap();

        assert!(!dropped.load(Ordering::SeqCst));

        // Replace with a fresh logger; this should drop the previous one.
        set_logger(DropFlag {
            dropped: Arc::new(AtomicBool::new(false)),
            log_count: Arc::new(AtomicUsize::new(0)),
        })
        .unwrap();

        assert!(
            dropped.load(Ordering::SeqCst),
            "previous logger should be dropped when the logger is replaced"
        );
    }

    #[test]
    fn verbosity_round_trips_known_values() {
        assert_eq!(
            Verbosity::from_raw(raw::verbosity_NIX_LVL_ERROR),
            Verbosity::Error
        );
        assert_eq!(
            Verbosity::from_raw(raw::verbosity_NIX_LVL_VOMIT),
            Verbosity::Vomit
        );
    }
}
