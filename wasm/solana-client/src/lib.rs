#[allow(clippy::result_large_err)]
mod client;
mod error;
mod provider;
mod types;

#[cfg(feature = "pubsub")]
mod pubsub;

// For root level imports
pub use {
    client::WasmClient, error::Error as ClientError, error::ErrorKind as ClientErrorKind,
    solana_rpc_client_api::request::RpcRequest as ClientRequest, types::*,
};

pub type ClientResult<T> = Result<T, ClientError>;
