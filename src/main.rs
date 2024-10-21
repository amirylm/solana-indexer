use std::sync::Arc;

// use std::sync::mpsc::Receiver;
use crossbeam_channel::{select, Receiver};
use futures_util::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::{RpcLogsResponse, SlotInfo},
};
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use tokio::sync::RwLock;

// The indexer consists of the following components:
// - **Solana RPC client**: responsible for interacting with the Solana blockchain
// - **Slot subscriber**: subscribes to new slots ([slot_subscribe](https://solana.com/docs/rpc/websocket/slotsubscribe)) using websocket and notifies block & log pollers upon new slot/s
// - **Block poller**: polls for new blocks ([get_blocks](https://solana.com/docs/rpc/#getconfirmedblocks)) and fetch corresponding information ([get_block](https://solana.com/docs/rpc/#getblock)) for existing blocks
// - **Log subscriber**: subscribes to logs ([logs_subscribe](https://solana.com/docs/rpc/websocket/logssubscribe)) using websocket and notifies log poller upon new log for some address.
// NOTE: using `logs_subscribe` is considered to be quite brittle and unreliable, thus we also use slot subscription to initiate log polling.
// - **Log poller**: continuously polls for logs by looping over txs of registered addresses ([get_signatures_for_address](https://solana.com/docs/rpc/http/getsignaturesforaddress)) and fetches the tx for each signature ([get_transaction](https://solana.com/docs/rpc/#gettransaction)) to extract logs.
//   - Additionally, it also fetches specific tx ([get_transaction](https://solana.com/docs/rpc/#gettransaction)) based on notification from log subscriber.

struct SlotSubscriber {
    ws_url: String,
    is_running: Arc<RwLock<bool>>,
}

impl SlotSubscriber {
    pub fn new(ws_url: &str) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn close(&self) {
        *self.is_running.write().await = false;
    }

    pub async fn run(&self) -> Result<Receiver<SlotInfo>, Box<dyn std::error::Error>> {
        let ps_client = PubsubClient::new(&self.ws_url).await?;
        let (sender, receiver) = crossbeam_channel::bounded(32);
        *self.is_running.write().await = true;
        let is_running_flag = self.is_running.clone();
        tokio::spawn(async move {
            let (mut slot_stream, unsubscriber) = ps_client.slot_subscribe().await.unwrap();
            while let Some(slot_info) = slot_stream.next().await {
                sender.send(slot_info).unwrap();
                let is_running = is_running_flag.read().await;
                if !*is_running {
                    break;
                }
            }
            unsubscriber().await;
        });
        Ok(receiver)
    }
}

struct LogSubscriber {
    ws_url: String,
    is_running: Arc<RwLock<bool>>,

    addrs: Vec<String>,
}

impl LogSubscriber {
    pub fn new(ws_url: &str, addrs: Vec<String>) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            is_running: Arc::new(RwLock::new(false)),
            addrs,
        }
    }

    pub async fn close(&self) {
        *self.is_running.write().await = false;
    }

    pub async fn run(&self) -> Result<Receiver<RpcLogsResponse>, Box<dyn std::error::Error>> {
        let (sender, receiver) = crossbeam_channel::bounded(32);
        *self.is_running.write().await = true;
        let is_running_flag = self.is_running.clone();
        let addrs = self.addrs.clone();
        let ws_url = self.ws_url.clone();
        for addr in addrs {
            let ws_url = ws_url.clone();
            let sender = sender.clone();
            let is_running_flag = is_running_flag.clone();
            tokio::spawn(async move {
                if let Err(e) = async {
                    println!("Subscribing to logs for address: {}", addr);
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
                        let is_running = is_running_flag.read().await;
                        if !*is_running {
                            break;
                        }
                    }
                    unsubscriber().await;
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
                .await
                {
                    eprintln!("Error subscribing to logs for address {}: {:?}", addr, e);
                }
            });
        }

        Ok(receiver)
    }
}

struct BlockPoller {
    http_url: String,
    is_running: Arc<RwLock<bool>>,
}

impl BlockPoller {
    pub fn new(http_url: &str) -> Self {
        Self {
            http_url: http_url.to_string(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn close(&self) {
        *self.is_running.write().await = false;
    }

    pub async fn run(
        &self,
        receiver: Receiver<SlotInfo>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

struct LogPoller {
    http_url: String,
    is_running: Arc<RwLock<bool>>,
}

impl LogPoller {
    pub fn new(http_url: &str) -> Self {
        Self {
            http_url: http_url.to_string(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn close(&self) {
        *self.is_running.write().await = false;
    }

    pub async fn run(
        &self,
        slot_receiver: Receiver<SlotInfo>,
        log_receiver: Receiver<RpcLogsResponse>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ws_url = "ws://127.0.0.1:8900";

    let slot_sub = SlotSubscriber::new(ws_url);
    let slot_receiver = slot_sub.run().await?;

    let log_sub = LogSubscriber::new(
        ws_url,
        vec![
            "2odvnPqk4HpXjjLEpPDLuzDHxnspVhvz59qPQAG5tCYX".to_string(),
            "DuVT5fpgy1thGWbHMBcniik6bkzdTt5LZPN4VmuPWTvE".to_string(),
        ],
    );
    let log_receiver = log_sub.run().await?;

    // tokio::spawn(async move {
    //     tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    //     slot_sub.close().await;
    //     log_sub.close().await;
    // });

    loop {
        select! {
            recv(slot_receiver) -> slot_info => {
                match slot_info {
                    Ok(info) => println!("Received slot info: {:?}", info),
                    Err(e) => {
                        eprintln!("Error receiving slot info: {:?}", e);
                        break
                    },
                }
            },
            recv(log_receiver) -> logs_info => {
                match logs_info {
                    Ok(info) => println!("Received logs info: {:?}", info),
                    Err(e) => {
                        eprintln!("Error receiving logs: {:?}", e);
                        break
                    },
                }
            }
        }
    }

    slot_sub.close().await;
    log_sub.close().await;

    Ok(())
}
