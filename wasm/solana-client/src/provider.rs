use reqwest::{
    header::{self, CONTENT_TYPE, RETRY_AFTER},
    StatusCode,
};
use serde_json::Value;
use solana_rpc_client_api::{
    custom_error,
    error_object::RpcErrorObject,
    request::{RpcError, RpcRequest, RpcResponseErrorData},
    response::RpcSimulateTransactionResult,
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::time::sleep;

use crate::ClientResult;

pub struct HttpProvider {
    client: reqwest::Client,
    url: String,
    request_id: AtomicU64,
}

impl HttpProvider {
    pub fn new(url: impl ToString) -> Self {
        Self::new_with_timeout(url, Duration::from_secs(30))
    }

    pub fn new_with_timeout(url: impl ToString, timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .default_headers(Self::default_headers())
                .timeout(timeout)
                .pool_idle_timeout(timeout)
                .build()
                .expect("invalid rpc client"),
            url: url.to_string(),
            request_id: AtomicU64::new(0),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn default_headers() -> header::HeaderMap {
        let mut default_headers = header::HeaderMap::new();
        default_headers.append(
            header::HeaderName::from_static("solana-client"),
            header::HeaderValue::from_str(
                format!("rust/{}", solana_version::Version::default()).as_str(),
            )
            .unwrap(),
        );
        default_headers
    }
}

impl HttpProvider {
    pub async fn send(&self, request: RpcRequest, params: Value) -> ClientResult<Value> {
        let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let request_json = request.build_request_json(request_id, params).to_string();

        let mut too_many_requests_retries = 5;
        loop {
            let response = {
                let client = self.client.clone();
                let request_json = request_json.clone();
                client
                    .post(&self.url)
                    .header(CONTENT_TYPE, "application/json")
                    .body(request_json)
                    .send()
                    .await
            }?;

            if !response.status().is_success() {
                if response.status() == StatusCode::TOO_MANY_REQUESTS
                    && too_many_requests_retries > 0
                {
                    let mut duration = Duration::from_millis(500);
                    if let Some(retry_after) = response.headers().get(RETRY_AFTER) {
                        if let Ok(retry_after) = retry_after.to_str() {
                            if let Ok(retry_after) = retry_after.parse::<u64>() {
                                if retry_after < 120 {
                                    duration = Duration::from_secs(retry_after);
                                }
                            }
                        }
                    }

                    too_many_requests_retries -= 1;
                    sleep(duration).await;
                    continue;
                }
                return Err(response.error_for_status().unwrap_err().into());
            }

            let mut json = response.json::<Value>().await?;
            if json["error"].is_object() {
                return match serde_json::from_value::<RpcErrorObject>(json["error"].clone()) {
                    Ok(rpc_error_object) => {
                        let data = match rpc_error_object.code {
                                    custom_error::JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE => {
                                        match serde_json::from_value::<RpcSimulateTransactionResult>(json["error"]["data"].clone()) {
                                            Ok(data) => RpcResponseErrorData::SendTransactionPreflightFailure(data),
                                            Err(_) => {
                                                RpcResponseErrorData::Empty
                                            }
                                        }
                                    },
                                    custom_error::JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY => {
                                        match serde_json::from_value::<custom_error::NodeUnhealthyErrorData>(json["error"]["data"].clone()) {
                                            Ok(custom_error::NodeUnhealthyErrorData {num_slots_behind}) => RpcResponseErrorData::NodeUnhealthy {num_slots_behind},
                                            Err(_err) => {
                                                RpcResponseErrorData::Empty
                                            }
                                        }
                                    },
                                    _ => RpcResponseErrorData::Empty
                                };

                        Err(RpcError::RpcResponseError {
                            code: rpc_error_object.code,
                            message: rpc_error_object.message,
                            data,
                        }
                        .into())
                    }
                    Err(err) => Err(RpcError::RpcRequestError(format!(
                        "Failed to deserialize RPC error response: {} [{}]",
                        serde_json::to_string(&json["error"]).unwrap(),
                        err
                    ))
                    .into()),
                };
            }
            return Ok(json["result"].take());
        }
    }
}

pub enum Provider {
    Http(HttpProvider),
}

impl Provider {
    pub fn url(&self) -> &str {
        match self {
            Provider::Http(http_provider) => http_provider.url(),
        }
    }

    pub async fn send(&self, request: RpcRequest, params: Value) -> ClientResult<Value> {
        match self {
            Provider::Http(http_provider) => http_provider.send(request, params).await,
        }
    }
}
