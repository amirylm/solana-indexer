

use std::sync::{Arc, atomic::{self, AtomicBool, AtomicU8}};
use crossbeam_channel::Receiver;
use futures_util::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_response::SlotInfo,
};

pub struct SlotSubscriber {
    ws_url: String,
    is_running: Arc<AtomicBool>,
    subscribers: Arc<AtomicU8>,
}

impl SlotSubscriber {
    pub fn new(ws_url: &str) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            is_running: Arc::new(AtomicBool::new(false)),
            subscribers: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn subscribe(&self) {
        self.subscribers.fetch_add(1, atomic::Ordering::Relaxed);
    }

    pub async fn close(&self) {
        self.is_running.store(false, atomic::Ordering::Relaxed);
    }

    pub async fn run(&self) -> Result<Receiver<SlotInfo>, Box<dyn std::error::Error>> {
        let ps_client = PubsubClient::new(&self.ws_url).await?;
        let (sender, receiver) = crossbeam_channel::bounded(32);
        self.is_running.store(true, atomic::Ordering::Relaxed);
        let is_running = self.is_running.clone();
        let subscribers = self.subscribers.clone();
        println!("[slot_subscriber] Subscribing to slots");
        tokio::spawn(async move {
            let (mut slot_stream, unsubscriber) = ps_client.slot_subscribe().await.unwrap();
            while let Some(slot_info) = slot_stream.next().await {
                if slot_info.slot % 10 == 0 {
                    println!("[slot_subscriber] Received new slot: {:?}", slot_info);
                }
                for _ in 0..subscribers.load(atomic::Ordering::Relaxed) {
                    sender.send(slot_info.clone()).unwrap();
                }
                if !is_running.load(atomic::Ordering::Relaxed) {
                    break;
                }
            }
            unsubscriber().await;
            println!("[slot_subscriber] Unsubscribed");
        });
        Ok(receiver)
    }
}
