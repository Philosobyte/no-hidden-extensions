use thiserror::Error;

#[derive(Error, Debug)]
#[error("The current executable path could not be converted into a UTF-8 string")]
pub(crate) struct NonUtf8ExecutablePathError;

#[derive(Error, Debug)]
pub(crate) enum RegistryOpsError {
    #[error("Failed to look up registry data for key {key} and value {value} with error")]
    FailedToGetValueData {
        key: String,
        value: String,
        source: std::io::Error,
    }
}

#[derive(Error, Debug)]
#[error("Failed to find a running process with name {0}")]
pub(crate) struct ProcessNotFoundError(pub(crate) String);

#[derive(Error, Debug)]
#[error("Failed to restart Windows Explorer in order for it to pick up registry changes")]
pub(crate) struct UnableToRestartWindowsExplorer;

#[derive(Error, Debug)]
pub(crate) enum IconLoadingError {
    #[error("Failed to load this program's icon")]
    FailedToLoadIconBytes(#[source] anyhow::Error),

    #[error("Failed to construct this program's tray icon")]
    FailedToConstructTrayIcon(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Failed to construct this program's window icon")]
    FailedToConstructWindowIcon(#[source] Box<dyn std::error::Error + Send + Sync>)
}
