use std::time::Duration;

use serde::Serialize;
use solana_commitment_config::CommitmentConfig;
use solana_hash::Hash;
use solana_message::{v0, Message as LegacyMessage};
use solana_signature::Signature;
use solana_transaction::{uses_durable_nonce, versioned::VersionedTransaction, Transaction};

#[derive(Default)]
pub struct RpcClientConfig {
    pub commitment_config: CommitmentConfig,
    pub confirm_transaction_initial_timeout: Option<Duration>,
}

impl RpcClientConfig {
    pub fn with_commitment(commitment_config: CommitmentConfig) -> Self {
        RpcClientConfig {
            commitment_config,
            ..Self::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct GetConfirmedSignaturesForAddress2Config {
    pub before: Option<Signature>,
    pub until: Option<Signature>,
    pub limit: Option<usize>,
    pub commitment: Option<CommitmentConfig>,
}

/// Trait used to add support for versioned messages to RPC APIs while
/// retaining backwards compatibility
pub trait SerializableMessage: Serialize {}
impl SerializableMessage for LegacyMessage {}
impl SerializableMessage for v0::Message {}

/// Trait used to add support for versioned transactions to RPC APIs while
/// retaining backwards compatibility
pub trait SerializableTransaction: Serialize {
    fn get_signature(&self) -> &Signature;
    fn get_recent_blockhash(&self) -> &Hash;
    fn uses_durable_nonce(&self) -> bool;
}
impl SerializableTransaction for Transaction {
    fn get_signature(&self) -> &Signature {
        &self.signatures[0]
    }
    fn get_recent_blockhash(&self) -> &Hash {
        &self.message.recent_blockhash
    }
    fn uses_durable_nonce(&self) -> bool {
        uses_durable_nonce(self).is_some()
    }
}
impl SerializableTransaction for VersionedTransaction {
    fn get_signature(&self) -> &Signature {
        &self.signatures[0]
    }
    fn get_recent_blockhash(&self) -> &Hash {
        self.message.recent_blockhash()
    }
    fn uses_durable_nonce(&self) -> bool {
        self.uses_durable_nonce()
    }
}
