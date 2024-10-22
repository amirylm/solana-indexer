use std::sync::atomic::{self, AtomicU64};
use crossbeam_channel::{select, Receiver};
use solana_client::{
    rpc_client::RpcClient,
    rpc_response::SlotInfo,
};

pub struct BlockPoller {
    http_url: String,

    finality: u64,
    batch_size: u64,
    last_slot: AtomicU64,
    last_fetched: AtomicU64,
}

impl BlockPoller {
    pub fn new(http_url: &str, finality: u64, batch_size: u64) -> Self {
        Self {
            http_url: http_url.to_string(),
            finality,
            batch_size,
            last_slot: AtomicU64::new(0),
            last_fetched: AtomicU64::new(0),
        }
    }

    pub async fn run(&self, signal: Receiver<bool>, receiver: Receiver<SlotInfo>) {
        println!("[block_poller] Starting block poller");
        loop {
            select! {
                recv(receiver) -> slot_info => {
                    match slot_info {
                        Ok(info) => {
                            let last_slot = self.last_slot.load(atomic::Ordering::Relaxed);
                            if last_slot == 0 {
                                // workaround to avoid syncing old blocks
                                self.last_slot.store(info.slot-self.finality, atomic::Ordering::Relaxed);
                                self.last_fetched.store(info.slot-self.finality, atomic::Ordering::Relaxed);
                                continue;
                            }
                            if info.slot <= last_slot {
                                continue; // skip known slots
                            }
                            self.last_slot.store(info.slot, atomic::Ordering::Relaxed);
                            let last_fetched = self.last_fetched.load(atomic::Ordering::Relaxed);
                            let mut top = last_fetched + self.batch_size;
                            if top > info.slot - self.finality {
                                top = info.slot - self.finality;
                            }
                            if top <= last_fetched {
                                continue;
                            }
                            let client = RpcClient::new(self.http_url.clone());
                            match client.get_blocks(last_fetched.into(), top.into()) {
                                Ok(blocks) => {
                                    println!("[block_poller] Got {} blocks in slots [{}, {}]", blocks.len(), last_fetched, top);
                                    if !blocks.is_empty() {
                                        self.last_fetched.store(top, atomic::Ordering::Relaxed);
                                    }
                                    for block in blocks {
                                        match client.get_block(block) {
                                            Ok(block_info) => {
                                                println!("[block_poller] Fetched block, height {:?}, hash {:?}", block_info.block_height, block_info.blockhash);
                                            },
                                            Err(e) => {
                                                eprintln!("[block_poller] Error fetching block: {:?}", e);
                                                // return Err(e.into());
                                            },
                                        }
                                    }
                                },
                                Err(e) => {
                                    eprintln!("[block_poller] Error fetching blocks: {:?}", e);
                                    // return Err(e.into());
                                },
                            }
                        },
                        Err(e) => {
                            eprintln!("[block_poller] Error receiving slot info: {:?}", e);
                            // return Err(e.into());
                        },
                    }
                },
                recv(signal) -> _ => {
                    println!("[block_poller] Received exit signal");
                    return;// Ok(());
                }
            }
        }
    }
}
