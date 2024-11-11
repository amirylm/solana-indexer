use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    RwLock,
};

use crate::rpc::RpcClientWrapper;

// Cursor is a helper struct to keep track of the last event that was read for an address.
struct Cursor {
    // slot is the slot of the last event that was read from the address
    slot: AtomicU64,
    // sig is the signature of the last event that was read from the address
    sig: RwLock<Signature>,
}

unsafe impl Send for Cursor {}

impl Cursor {
    // new creates a new Cursor with the given slot and signature
    fn new(slot: u64, sig: String) -> Self {
        match Signature::from_str(sig.as_str()) {
            Ok(signature) => Self {
                slot: AtomicU64::new(slot),
                sig: RwLock::new(signature),
            },
            Err(e) => {
                eprintln!("[cursor/new] Error parsing signature: {:?}", e);
                Self {
                    slot: AtomicU64::new(slot),
                    sig: RwLock::new(Signature::default()),
                }
            }
        }
    }

    // update updates the slot and signature of the Cursor
    fn update(&self, slot: u64, sig: Signature) {
        self.slot.store(slot, Ordering::Relaxed);
        let mut w = self.sig.write().unwrap();
        *w = sig;
    }

    // get_slot returns the slot of the Cursor
    fn get_slot(&self) -> u64 {
        self.slot.load(Ordering::Relaxed)
    }

    // get_sig returns the signature of the Cursor
    // fn get_sig(&self) -> Signature {
    //     let r = self.sig.read().unwrap();
    //     *r
    // }
}

// EventLoader loads log events from the Solana blockchain for a given program address.
pub struct EventLoader {
    client: RpcClientWrapper,
    head_cursor: Cursor,
    tail_cursor: Cursor,

    batch_size: usize,
    program_addr: String,
}

unsafe impl Send for EventLoader {}

impl EventLoader {
    // new creates a new EventLoader with the given client and cursors
    pub fn new(
        program_addr: String,
        batch_size: usize,
        client: RpcClientWrapper,
        head_slot: u64,
        head_sig: String,
        tail_slot: u64,
        tail_sig: String,
    ) -> Self {
        Self {
            client,
            head_cursor: Cursor::new(head_slot, head_sig),
            tail_cursor: Cursor::new(tail_slot, tail_sig),
            batch_size,
            program_addr,
        }
    }

    pub async fn poll(&self) -> Result<(), Box<dyn std::error::Error>> {
        let tail_slot = self.tail_cursor.get_slot();
        let head_slot = self.head_cursor.get_slot();
        let last_confirmed_slot = self
            .client
            .get_slot(Some(CommitmentConfig::confirmed()))
            .await?;
        let last_finalized_slot = self
            .client
            .get_slot(Some(CommitmentConfig::finalized()))
            .await?;
        println!(
            "[event_loader/poll] Polling for addr {} with head_slot={}, tail_slot={}, last_confirmed_slot={}, last_finalized_slot={}",
            self.program_addr, head_slot, tail_slot, last_confirmed_slot, last_finalized_slot
        );
        match self.backfill(last_finalized_slot).await {
            Ok(_) => {
                println!("[event_loader/poll] Backfilled");
            }
            Err(e) => {
                eprintln!("[event_loader/poll] Error backfilling: {:?}", e);
                return Err(e);
            }
        }
        match self.load_confirmed_events(last_confirmed_slot).await {
            Ok(_) => {
                println!("[event_loader/poll] Loaded confirmed events");
            }
            Err(e) => {
                eprintln!(
                    "[event_loader/poll] Error loading confirmed events: {:?}",
                    e
                );
                return Err(e);
            }
        }
        Ok(())
    }

    // backfill events from the tail_cursor to the target slot
    pub async fn backfill(&self, target_slot: u64) -> Result<(), Box<dyn std::error::Error>> {
        let pk = Pubkey::from_str(self.program_addr.as_str())?;
        println!(
            "[event_loader/backfill] Backfilling for addr {} to slot {}",
            self.program_addr, target_slot
        );
        let mut tail_slot = self.tail_cursor.get_slot();
        while target_slot > tail_slot {
            match self
                .client
                .get_sigs_for_addr(
                    &pk,
                    tail_slot,
                    self.batch_size,
                    Some(CommitmentConfig::finalized()),
                    None,
                    None,
                )
                .await
            {
                Ok(mut txs) => {
                    txs.sort_by(|a, b| a.slot.cmp(&b.slot));
                    for tx_status in txs.iter() {
                        if tail_slot >= tx_status.slot {
                            break;
                        }
                        let sig = Signature::from_str(tx_status.signature.as_str())?;
                        println!(
                            "[event_loader/backfill] Visiting tx (slot={}, sig={}, addr={})",
                            tx_status.slot,
                            tx_status.signature.clone(),
                            self.program_addr.clone()
                        );
                        match self
                            .client
                            .get_tx(&sig, Some(CommitmentConfig::finalized()))
                            .await
                        {
                            Ok(tx) => {
                                let logs = tx
                                    .transaction
                                    .meta
                                    .unwrap()
                                    .log_messages
                                    .unwrap_or(Vec::new());
                                self.process_finalized_logs(
                                    tx.slot,
                                    tx_status.signature.clone(),
                                    logs,
                                );
                            }
                            Err(e) => {
                                eprintln!("[event_loader/backfill] Error fetching tx: {:?}", e);
                                return Err(e.into());
                            }
                        }
                    }
                    let last_tx = txs.last().unwrap();

                    if tail_slot < last_tx.slot {
                        tail_slot = last_tx.slot;
                        println!(
                            "[event_loader/backfill] Updating tail_cursor to (slot={}, sig={})",
                            last_tx.slot,
                            last_tx.signature.as_str()
                        );
                        self.tail_cursor.update(
                            last_tx.slot,
                            Signature::from_str(last_tx.signature.as_str())?,
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[event_loader/backfill] Error fetching txs: {:?}", e);
                    return Err(e.into());
                }
            };
        }
        Ok(())
    }

    // load_events loads events from the head_cursor to the target slot
    pub async fn load_confirmed_events(
        &self,
        target_slot: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pk = Pubkey::from_str(self.program_addr.as_str())?;
        let mut head_slot = self.head_cursor.get_slot();
        while target_slot > head_slot {
            match self
                .client
                .get_sigs_for_addr(
                    &pk,
                    head_slot,
                    self.batch_size,
                    Some(CommitmentConfig::confirmed()),
                    None,
                    None,
                )
                .await
            {
                Ok(mut txs) => {
                    txs.sort_by(|a, b| a.slot.cmp(&b.slot));
                    for tx_status in txs.iter() {
                        let sig = Signature::from_str(tx_status.signature.as_str())?;
                        println!(
                            "[event_loader/load_confirmed_events] Visiting tx (slot={}, sig={}, addr={})",
                            tx_status.slot,
                            tx_status.signature.clone(),
                            self.program_addr.clone()
                        );
                        match self
                            .client
                            .get_tx(&sig, Some(CommitmentConfig::confirmed()))
                            .await
                        {
                            Ok(tx) => {
                                let logs = tx
                                    .transaction
                                    .meta
                                    .unwrap()
                                    .log_messages
                                    .unwrap_or(Vec::new());
                                self.process_confirmed_logs(
                                    tx.slot,
                                    tx_status.signature.clone(),
                                    logs,
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "[event_loader/load_confirmed_events] Error fetching tx: {:?}",
                                    e
                                );
                                return Err(e.into());
                            }
                        }
                    }
                    let last_tx = txs.last().unwrap();
                    if head_slot < last_tx.slot {
                        head_slot = last_tx.slot;
                        println!(
                        "[event_loader/load_confirmed_events] Updating head_cursor to (slot={}, sig={})",
                        last_tx.slot,
                        last_tx.signature.as_str()
                    );
                        self.head_cursor.update(
                            last_tx.slot,
                            Signature::from_str(last_tx.signature.as_str())?,
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[event_loader/load_confirmed_events] Error fetching txs: {:?}",
                        e
                    );
                    return Err(e.into());
                }
            };
        }
        Ok(())
    }

    fn process_finalized_logs(&self, slot: u64, sig: String, logs: Vec<String>) {
        let addr = self.program_addr.as_str();
        println!(
            "[event_loader/process_finalized_logs] Processing {} finalized logs for addr {} on slot {} and sig {}",
            logs.len(),
            addr,
            slot,
            sig.clone()
        );
        logs.iter().for_each(|log| {
            if let Some(parsed_log) = parse_log(log, addr) {
                println!(
                    "[event_loader/process_finalized_logs] Parsed finalized log: {:?}",
                    parsed_log
                );
            } else {
                println!(
                    "[event_loader/process_finalized_logs] Log with unknown structure: {}",
                    log
                );
            }
        });
    }

    fn process_confirmed_logs(&self, slot: u64, sig: String, logs: Vec<String>) {
        let addr = self.program_addr.as_str();
        println!(
            "[event_loader/process_confirmed_logs] Processing {} confirmed logs for addr {} on slot {} and sig {}",
            logs.len(),
            addr,
            slot,
            sig.clone()
        );
        logs.iter().for_each(|log| {
            if let Some(parsed_log) = parse_log(log, addr) {
                println!(
                    "[event_loader/process_confirmed_logs] Parsed confirmed log: {:?}",
                    parsed_log
                );
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogType {
    ProgramInvoke,
    ProgramLog,
    ProgramLogInstruction,
    ProgramData,
    ProgramConsumed,
    ProgramResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolLog {
    pub addr: String,
    pub data: String,
    pub log_type: LogType,
}

// parse logs from a transaction, supports:
// - Program (\w*) invoke \[(\d)\]: Program J1zQwrBNBngz26jRPNWsUSZMHJwBwpkoDitXRV95LdK4 invoke [1]
// - Program log: (Instruction: (.*)|.*): Program log: Instruction: CreateLog
// - Program data: (.*): Program data: HDQnaQjSWwkNAAAASGVsbG8sIFdvcmxkISoAAAAAAAAA // base64 encoded; borsh encoded with identifier
// - Program \w* consumed (\d*) (.*): Program J1zQwrBNBngz26jRPNWsUSZMHJwBwpkoDitXRV95LdK4 consumed 1477 of 200000 compute units
// - Program \w* (success|failed): Program J1zQwrBNBngz26jRPNWsUSZMHJwBwpkoDitXRV95LdK4 success
pub fn parse_log(log: &str, addr: &str) -> Option<SolLog> {
    let re = regex::Regex::new(r"Program log: (Instruction: (.*)|.*)").unwrap();
    if let Some(caps) = re.captures(log) {
        if let Some(instruction) = caps.get(2) {
            return Some(SolLog {
                addr: addr.to_string(),
                data: instruction.as_str().to_string(),
                log_type: LogType::ProgramLogInstruction,
            });
        } else if let Some(data) = caps.get(1) {
            return Some(SolLog {
                addr: addr.to_string(),
                data: data.as_str().to_string(),
                log_type: LogType::ProgramLog,
            });
        }
    }
    None
}
