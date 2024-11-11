use serde_json::json;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcSignaturesForAddressConfig, RpcTransactionConfig},
    rpc_request::RpcRequest,
    rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone, serde::Deserialize, serde::Serialize)]
pub enum RpcError {
    #[error("failed to get slot: {0}")]
    GetSlotError(String),
    #[error("failed to get sigs for addr {0}: {1}")]
    GetSigsForAddrError(String, String),
    #[error("failed to get tx for sig {0}: {1}")]
    GetTxError(String, String),
    #[error("failed to send {0}: {1}")]
    SendError(String, String),
}

struct EndPoint {
    client: RpcClient,
}

unsafe impl Send for EndPoint {}

impl EndPoint {
    fn new(url: String) -> Self {
        Self {
            client: RpcClient::new(url),
        }
    }
}

// RpcClientWrapper is a wrapper around RpcClient that allows for multiple servers
// and abstracts the api for the user
pub struct RpcClientWrapper {
    endpoint: EndPoint,
}

impl RpcClientWrapper {
    pub fn new(url: String) -> Self {
        Self {
            endpoint: EndPoint::new(url),
        }
    }

    pub async fn get_slot(
        &self,
        commitment_config: Option<CommitmentConfig>,
    ) -> Result<u64, RpcError> {
        match self
            .endpoint
            .client
            .get_slot_with_commitment(commitment_config.unwrap_or(CommitmentConfig::finalized()))
            .await
        {
            Ok(slot) => Ok(slot),
            Err(err) => Err(RpcError::GetSlotError(err.to_string())),
        }
    }

    pub async fn get_sigs_for_addr(
        &self,
        pk: &Pubkey,
        min_context_slot: u64,
        limit: usize,
        commitment_config: Option<CommitmentConfig>,
        until: Option<Signature>,
        before: Option<Signature>,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>, RpcError> {
        let commitment_cfg = commitment_config.unwrap_or(CommitmentConfig::finalized());
        let config = RpcSignaturesForAddressConfig {
            before: before.map(|signature| signature.to_string()),
            until: until.map(|signature| signature.to_string()),
            limit: Some(limit),
            commitment: Some(commitment_cfg),
            min_context_slot: Some(min_context_slot),
        };
        match self
            .endpoint
            .client
            .send(
                RpcRequest::GetSignaturesForAddress,
                json!([pk.to_string(), config]),
            )
            .await
        {
            Ok(sigs) => Ok(sigs),
            Err(err) => Err(RpcError::GetSigsForAddrError(
                pk.to_string(),
                err.to_string(),
            )),
        }
    }

    pub async fn get_tx(
        &self,
        sig: &Signature,
        commitment_config: Option<CommitmentConfig>,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, RpcError> {
        let commitment_cfg = commitment_config.unwrap_or(CommitmentConfig::finalized());
        let sig_str = sig.to_string();
        match self
            .endpoint
            .client
            .get_transaction_with_config(
                sig,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: Some(commitment_cfg),
                    max_supported_transaction_version: None,
                },
            )
            .await
        {
            Ok(tx) => Ok(tx),
            Err(err) => Err(RpcError::GetTxError(sig_str, err.to_string())),
        }
    }

    pub async fn send(
        &self,
        req: RpcRequest,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        match self.endpoint.client.send(req, params.clone()).await {
            Ok(res) => Ok(res),
            Err(err) => Err(RpcError::SendError(req.to_string(), err.to_string())),
        }
    }
}
