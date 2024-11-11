use dotenv::dotenv;
use std::{result::Result, sync::Arc, time::Duration};
use tokio::{signal, sync::oneshot, task, time};

use solana_indexer::{log_events::EventLoader, rpc::RpcClientWrapper};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let rpc_url = get_env("SOL_RPC", "http://127.0.0.1:8899");
    let program_addr = get_env("SOL_PROGRAM", format!("{:0>64x}", 0).as_str());
    let txs_batch_size = get_env("SOL_BATCH_SIZE", "100").parse::<usize>()?;
    let block_time = get_env("SOL_BLOCK_TIME", "5000").parse::<u64>()?; // ms
    let head_slot = get_env("SOL_HEAD_SLOT", "0").parse::<u64>()?;
    let head_sig = get_env("SOL_HEAD_SIG", format!("0x{:0>64x}", 0).as_str());
    let tail_slot = get_env("SOL_TAIL_SLOT", "0").parse::<u64>()?;
    let tail_sig = get_env("SOL_TAIL_SIG", format!("0x{:0>64x}", 0).as_str());

    let client = RpcClientWrapper::new(rpc_url.clone());

    let loader = Arc::new(EventLoader::new(
        program_addr,
        txs_batch_size,
        client,
        head_slot,
        head_sig,
        tail_slot,
        tail_sig,
    ));

    let (tx, shutdown) = oneshot::channel();
    tokio::spawn(async move {
        if signal::ctrl_c().await.is_ok() {
            tx.send(()).unwrap()
        }
    });

    let e_loader = loader.clone();
    let main_handle = task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(block_time));

        tokio::select! {
            _ = async {
                loop {
                    interval.tick().await;
                    match e_loader.poll().await {
                        Ok(_) => {
                            println!("polled");
                        },
                        Err(e) => {
                            eprintln!("failed to poll: {}", e);
                        }
                    }
                }
            } => {
                println!("exiting");
            },
            _ = shutdown => {
                println!("shutting down");
            }
        };
    });

    main_handle.await?;

    Ok(())
}

fn get_env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
