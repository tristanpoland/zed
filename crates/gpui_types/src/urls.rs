/// URL operations supplied by a platform implementation.
pub trait PlatformUrlSpi {
    /// The task type used by the platform runtime.
    type Task<T>;

    /// The error type returned by the platform runtime.
    type Error;

    /// Directs the platform's default browser to open the given URL.
    fn open_url(&self, url: &str);

    /// Registers a handler to be invoked when the platform instructs the application
    /// to open one or more URLs.
    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>);

    /// Registers the given URL scheme to be opened by the current app.
    fn register_url_scheme(&self, url: &str) -> Self::Task<Result<(), Self::Error>>;
}
