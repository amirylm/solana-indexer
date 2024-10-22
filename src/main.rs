use crossbeam_channel::select;
use solana_indexer::{
    block_poller::BlockPoller,
    log_poller::LogPoller,
    log_subscriber::LogSubscriber,
    slot_subscriber::SlotSubscriber,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_url = "http://127.0.0.1:8899";
    let ws_url = "ws://127.0.0.1:8900";
    let addrs = vec![
        "9RfJMjDmFKp8Ueza94VUT66JTwWJLEg8rs5GTj2tB1h9".to_string(),
        "qnGqATVVx3AZqaT6M3h6GwG2TrW2Ug4iyZ4aoaaH33E".to_string(),
    ];

    let slot_sub = SlotSubscriber::new(ws_url);
    let slot_receiver = slot_sub.run().await?;

    let log_sub = LogSubscriber::new(ws_url, addrs.clone());
    let log_receiver = log_sub.run().await?;

    let (signal_sender, signal) = crossbeam_channel::bounded(1);
    let block_poller_signal = signal.clone();
    let block_poller_slot_receiver = slot_receiver.clone();
    slot_sub.subscribe();
    let bp_http_url = http_url.to_string().clone();
    tokio::spawn(async move {
        let block_poller = BlockPoller::new(bp_http_url.as_str(), 1000, 10);
        block_poller
            .run(block_poller_signal, block_poller_slot_receiver)
            .await;
    });

    let log_poller_signal = signal.clone();
    let log_poller_slot_receiver = slot_receiver.clone();
    slot_sub.subscribe();
    let log_poller_log_receiver = log_receiver.clone();
    let lp_http_url = http_url.to_string().clone();
    tokio::spawn(async move {
        let log_poller = LogPoller::new(lp_http_url.as_str(), 10, addrs.clone());
        log_poller
            .run(
                log_poller_signal,
                log_poller_slot_receiver,
                log_poller_log_receiver,
            )
            .await;
    });

    slot_sub.subscribe();
    loop {
        select! {
            recv(slot_receiver) -> slot_info => {
                match slot_info {
                    Ok(_) => {
                        // println!("Received slot info: {:?}", info);
                    },
                    Err(e) => {
                        eprintln!("Error receiving slot info: {:?}", e);
                        break
                    },
                }
            },
            recv(log_receiver) -> logs_info => {
                match logs_info {
                    Ok(info) => {
                        println!("Received logs info: {:?}", info);
                    },
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
    signal_sender.send(true).unwrap();

    Ok(())
}
