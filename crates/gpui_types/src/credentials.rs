/// Credential operations supplied by a platform implementation.
pub trait PlatformCredentialsSpi {
    /// The task type used by the platform runtime.
    type Task<T>;

    /// The error type returned by the platform runtime.
    type Error;

    /// Writes credentials to the platform's secure credential store.
    fn write_credentials(
        &self,
        url: &str,
        username: &str,
        password: &[u8],
    ) -> Self::Task<Result<(), Self::Error>>;

    /// Reads credentials from the platform's secure credential store.
    fn read_credentials(
        &self,
        url: &str,
    ) -> Self::Task<Result<Option<(String, Vec<u8>)>, Self::Error>>;

    /// Deletes credentials from the platform's secure credential store.
    fn delete_credentials(&self, url: &str) -> Self::Task<Result<(), Self::Error>>;
}
