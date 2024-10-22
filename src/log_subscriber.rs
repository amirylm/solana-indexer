use std::sync::{Arc, atomic::{self, AtomicBool}};
use crossbeam_channel::Receiver;
use futures_util::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::RpcLogsResponse,
};
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};

pub struct LogSubscriber {
    ws_url: String,
    is_running: Arc<AtomicBool>,

    addrs: Vec<String>,
}

impl LogSubscriber {
    pub fn new(ws_url: &str, addrs: Vec<String>) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            is_running: Arc::new(AtomicBool::new(false)),
            addrs,
        }
    }

    pub async fn close(&self) {
        self.is_running.store(false, atomic::Ordering::Relaxed);
    }

    pub async fn run(&self) -> Result<Receiver<RpcLogsResponse>, Box<dyn std::error::Error>> {
        let (sender, receiver) = crossbeam_channel::bounded(32);
        self.is_running.store(true, atomic::Ordering::Relaxed);
        let addrs = self.addrs.clone();
        let ws_url = self.ws_url.clone();
        for addr in addrs {
            let ws_url = ws_url.clone();
            let sender = sender.clone();
            let is_running = self.is_running.clone();
            tokio::spawn(async move {
                if let Err(e) = async {
                    println!("[log_subscriber] Subscribing to logs for address: {}", addr);
                    let ps_client = PubsubClient::new(ws_url.as_str()).await?; // TODO: use a single client
                    let filter = RpcTransactionLogsFilter::Mentions(vec![addr.to_string()]);
                    let cfg = RpcTransactionLogsConfig {
                        commitment: Some(CommitmentConfig {
                            commitment: CommitmentLevel::Processed,
                        }),
                    };
                    let (mut slot_stream, unsubscriber) =
                        ps_client.logs_subscribe(filter, cfg).await?;
                    while let Some(logs_info) = slot_stream.next().await {
                        sender.send(logs_info.value).unwrap();
                        if !is_running.load(atomic::Ordering::Relaxed) {
                            break;
                        }
                    }
                    unsubscriber().await;
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
                .await
                {
                    eprintln!("[log_subscriber] Error subscribing to logs for address {}: {:?}", addr, e);
                }
            });
        }

        Ok(receiver)
    }
}
