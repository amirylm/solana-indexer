use std::str::FromStr;
use std::sync::atomic::{self, AtomicU64};
use crossbeam_channel::{select, Receiver};
use solana_client::{
    rpc_client::RpcClient,
    rpc_response::{RpcLogsResponse, SlotInfo},
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;

pub struct LogPoller {
    http_url: String,
    slot_backoff: u64,
    addrs: Vec<String>,
    last_slot: AtomicU64,
}

impl LogPoller {
    pub fn new(http_url: &str, slot_backoff: u64, addrs: Vec<String>) -> Self {
        Self {
            http_url: http_url.to_string(),
            slot_backoff,
            addrs,
            last_slot: AtomicU64::new(0),
        }
    }

    pub async fn run(
        &self,
        signal: Receiver<bool>,
        slot_receiver: Receiver<SlotInfo>,
        log_receiver: Receiver<RpcLogsResponse>,
    ) {
        let addrs = self.addrs.clone();
        let slot_backoff = self.slot_backoff;
        println!("[log_poller] Starting log poller");
        loop {
            select! {
                recv(log_receiver) -> logs_info => {
                    match logs_info {
                        Ok(info) => {
                            let client = RpcClient::new(self.http_url.clone());
                            let sig = Signature::from_str(info.signature.as_str()).unwrap();
                            match client.get_transaction(&sig, UiTransactionEncoding::Json) {
                                Ok(tx) => {
                                    println!("[log_poller] Fetched tx on new log: {:?}", tx);
                                },
                                Err(e) => {
                                    eprintln!("[log_poller] Error fetching tx: {:?}", e);
                                    // return Err(e.into());
                                },
                            }
                        },
                        Err(e) => {
                            eprintln!("[log_poller] Error receiving logs: {:?}", e);
                            // return Err(e.into());
                        },
                    }
                },
                recv(slot_receiver) -> slot_info => {
                    match slot_info {
                        Ok(info) => {
                            let last_slot = self.last_slot.load(atomic::Ordering::Relaxed);
                            println!("[log_poller] Received slot info: {:?}; last_slot: {}", info, last_slot);
                            if last_slot == 0 {
                                // workaround to avoid syncing old blocks
                                self.last_slot.store(info.slot - slot_backoff*100, atomic::Ordering::Relaxed);
                                continue;
                            }
                            // backoff some slots to avoid spamming the RPC node
                            if info.slot - last_slot < slot_backoff {
                                continue;
                            }
                            let client = RpcClient::new(self.http_url.clone());
                            for addr in addrs.clone() {
                                let pk = Pubkey::from_str(addr.as_str()).unwrap();
                                match client.get_signatures_for_address(&pk) {
                                    Ok(signatures) => {
                                        println!("[log_poller] Fetched {} signatures for address {:?}", signatures.len(), pk);
                                        for signature in signatures {
                                            let sig = Signature::from_str(signature.signature.as_str()).unwrap();
                                            match client.get_transaction(&sig, UiTransactionEncoding::Json) {
                                                Ok(tx) => {
                                                    println!("[log_poller] Fetched tx for account keys {:?}", tx.transaction.meta);
                                                },
                                                Err(e) => {
                                                    eprintln!("[log_poller] Error fetching tx: {:?}", e);
                                                    // return Err(e.into());
                                                },
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("[log_poller] Error fetching signatures: {:?}", e);
                                        // return Err(e.into());
                                    },
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("[log_poller] Error receiving slot info: {:?}", e);
                            // return Err(e.into());
                        },
                    }
                },
                recv(signal) -> _ => {
                    println!("[log_poller] Received exit signal");
                    // return Ok(());
                    return;
                }
            }
        }
        // Ok(())
    }
}
