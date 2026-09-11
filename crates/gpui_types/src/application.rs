use std::{ffi::OsString, path::PathBuf};

use crate::{
    clipboard::{ClipboardItem, ClipboardReadError, PlatformClipboardSpi},
    credentials::PlatformCredentialsSpi,
    keyboard::PlatformKeyboardSpi,
    notifications::PlatformSystemNotificationSpi,
    paths::PlatformPathSpi,
    platform::PlatformCursorSpi,
    urls::PlatformUrlSpi,
};

/// The application's lifecycle phase, as owned and reported by a mobile OS.
///
/// `Inactive` means visible but not receiving input (a system dialog on
/// top), while `Background` means not visible at all, with process death
/// possible at any time thereafter.
///
/// | Phase        | iOS                          | Android      |
/// |--------------|------------------------------|--------------|
/// | `Active`     | `didBecomeActive`            | `onResume`   |
/// | `Inactive`   | `willResignActive`           | `onPause`    |
/// | `Background` | `didEnterBackground`         | `onStop`     |
/// | `Foreground` | `willEnterForeground`        | `onStart`    |
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AppLifecyclePhase {
    /// Foreground and receiving input.
    Active,
    /// Foreground (visible) but not receiving input.
    Inactive,
    /// Not visible. The GPU surface may be destroyed while backgrounded and
    /// the process may be killed without further notice.
    Background,
    /// Becoming visible again, before input is restored.
    Foreground,
}

/// Application lifecycle and identity operations supplied by a platform implementation.
pub trait PlatformApplicationSpi {
    /// Starts the platform run loop and invokes the launch callback when the app is ready.
    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>);

    /// Quits the application through the platform's standard routine.
    fn quit(&self);

    /// Restarts the application with an optional executable path and arguments.
    fn restart(&self, binary_path: Option<PathBuf>, arguments: Vec<OsString>);

    /// Activates the application, optionally ignoring other applications.
    fn activate(&self, ignoring_other_apps: bool);

    /// Hides the application.
    fn hide(&self);

    /// Hides other applications.
    fn hide_other_apps(&self);

    /// Unhides other applications.
    fn unhide_other_apps(&self);

    /// Registers a callback invoked when the platform requests that the application quit.
    fn on_quit(&self, callback: Box<dyn FnMut() -> bool>);

    /// Registers a callback invoked when an already-running application is launched again.
    fn on_reopen(&self, callback: Box<dyn FnMut()>);

    /// Registers a callback invoked when the system wakes from sleep.
    fn on_system_wake(&self, callback: Box<dyn FnMut()>);

    /// Registers a callback invoked whenever the application's lifecycle phase changes.
    fn on_app_lifecycle(&self, _callback: Box<dyn FnMut(AppLifecyclePhase)>) {}

    /// Registers a callback invoked when the OS signals memory pressure.
    fn on_memory_warning(&self, _callback: Box<dyn FnMut()>) {}

    /// Sets the application's process-wide identity and user-visible name.
    fn set_app_identity(&self, _identifier: &str, _name: &str) {}
}

/// The application-level platform capabilities used by GPUI.
///
/// The capability traits provide the backend-neutral operation contracts. This
/// composition adds the remaining platform services that are specific to the
/// application boundary while keeping the runtime's task and clipboard image
/// types associated with its implementation.
pub trait PlatformServicesSpi:
    PlatformApplicationSpi
    + PlatformCursorSpi
    + PlatformClipboardSpi
    + PlatformCredentialsSpi
    + PlatformSystemNotificationSpi
    + PlatformUrlSpi
    + PlatformPathSpi
    + PlatformKeyboardSpi
{
    /// The task returned by asynchronous clipboard reads.
    type ClipboardTask<T>;

    /// Reads the clipboard, resolving once its contents are available.
    fn read_from_clipboard_async(
        &self,
    ) -> Self::ClipboardTask<
        Result<Option<ClipboardItem<<Self as PlatformClipboardSpi>::Image>>, ClipboardReadError>,
    >;

    /// Reads data from the primary selection buffer.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn read_from_primary(&self) -> Option<ClipboardItem<<Self as PlatformClipboardSpi>::Image>>;

    /// Writes data to the primary selection buffer.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn write_to_primary(&self, item: ClipboardItem<<Self as PlatformClipboardSpi>::Image>);

    /// Reads data from macOS's Find pasteboard.
    #[cfg(target_os = "macos")]
    fn read_from_find_pasteboard(
        &self,
    ) -> Option<ClipboardItem<<Self as PlatformClipboardSpi>::Image>>;

    /// Writes data to macOS's Find pasteboard.
    #[cfg(target_os = "macos")]
    fn write_to_find_pasteboard(&self, item: ClipboardItem<<Self as PlatformClipboardSpi>::Image>);

    /// Returns whether the platform file picker supports automatically hiding scrollbars.
    fn should_auto_hide_scrollbars(&self) -> bool;

    /// Returns the full pathname of the current application bundle.
    fn app_path(&self) -> anyhow::Result<PathBuf>;

    /// Returns the path of an auxiliary executable in the application bundle.
    fn path_for_auxiliary_executable(&self, name: &str) -> anyhow::Result<PathBuf>;

    /// Returns the name of the compositor in use, when applicable.
    fn compositor_name(&self) -> &'static str;
}
