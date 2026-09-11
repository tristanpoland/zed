use std::path::{Path, PathBuf};

/// The options that can be configured for a file dialog prompt.
#[derive(Clone, Debug)]
pub struct PathPromptOptions {
    /// Should the prompt allow files to be selected?
    pub files: bool,
    /// Should the prompt allow directories to be selected?
    pub directories: bool,
    /// Should the prompt allow multiple files to be selected?
    pub multiple: bool,
    /// The prompt to show to a user when selecting a path.
    pub prompt: Option<gpui_shared_string::SharedString>,
}

/// Path operations supplied by a platform implementation.
pub trait PlatformPathSpi {
    /// The task type used by the platform runtime.
    type Task<T>;

    /// The error type returned by the platform runtime.
    type Error;

    /// Displays a platform modal for selecting paths.
    fn prompt_for_paths(
        &self,
        options: PathPromptOptions,
    ) -> Self::Task<Result<Option<Vec<PathBuf>>, Self::Error>>;

    /// Displays a platform modal for selecting a new path where a file can be saved.
    fn prompt_for_new_path(
        &self,
        directory: &Path,
        suggested_name: Option<&str>,
    ) -> Self::Task<Result<Option<PathBuf>, Self::Error>>;

    /// Returns whether the platform file picker supports selecting a mix of files and directories.
    fn can_select_mixed_files_and_dirs(&self) -> bool;

    /// Reveals the specified path at the platform level, such as in Finder on macOS.
    fn reveal_path(&self, path: &Path);

    /// Opens the specified path with the system's default application.
    fn open_with_system(&self, path: &Path);
}
