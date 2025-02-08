use std::net::Ipv4Addr;
use std::str;
use std::time::Duration;

use tokio::process::Command;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const ECHO_TIMES: usize = 4;

enum Message {
    Start,
    Collect(String),
    End,
}

async fn ping(ip: &str) -> bool {
    let output = Command::new("ping")
        .arg("-n")
        .arg(ECHO_TIMES.to_string().as_str())
        .arg("-w")
        .arg("50")
        .arg(ip)
        .output()
        .await
        .expect("failed to execute process");
    let string = str::from_utf8(&output.stdout).unwrap();

    let count = string
        .lines()
        .filter(|&line| line.contains("Request timed out."))
        .count();

    // We send 4 echo request, if all is Request timed out. it means that IP has no device connected.
    count != ECHO_TIMES
}

async fn scan_subnet(starting_ip: &str, subnet_mask: &str, tx: UnboundedSender<Message>) {
    let mut tasks = Vec::new();
    let starting_ip: Ipv4Addr = starting_ip.parse().unwrap();
    let subnet_mask: Ipv4Addr = subnet_mask.parse().unwrap();
    let ip_range = get_ip_range(starting_ip, subnet_mask);

    tx.send(Message::Start).unwrap();

    for ip in ip_range {
        let ip = ip.to_string();
        let tx_inner = tx.clone();
        let task = tokio::spawn(async move {
            if ping(&ip).await {
                tx_inner.send(Message::Collect(ip)).unwrap();
            }
        });

        tasks.push(task);
    }

    for join in tasks {
        join.await.unwrap();
    }
    tx.send(Message::End).unwrap();
}

async fn monitor_connections(mut rx: UnboundedReceiver<Message>) {
    tokio::spawn(async move {
        let mut previous = Vec::<String>::new();
        let mut current = Vec::<String>::new();

        loop {
            select! {
                Some(msg) = rx.recv() => {
                    match msg {
                        Message::Start => {
                            current.clear();
                        },
                        Message::Collect(ip) => {
                            current.push(ip);
                        },
                        Message::End => {
                            current.sort();
                            let only_in_vec1: Vec<_> = previous.iter().filter(|&item| !current.contains(item)).collect();
                            let only_in_vec2: Vec<_> = current.iter().filter(|&item| !previous.contains(item)).collect();

                            println!("Current Connections: {:?}", current);
                            println!("Newly Connected: {:?}", only_in_vec2) ;
                            println!("Disconnected: {:?}", only_in_vec1) ;

                            previous = current.clone();
                        },
                    }
                }
            }
        }
    });
}

fn get_ip_range(starting_ip: Ipv4Addr, subnet_mask: Ipv4Addr) -> Vec<Ipv4Addr> {
    let ip_u32 = u32::from(starting_ip);
    let mask_u32 = u32::from(subnet_mask);

    let network = ip_u32 & mask_u32;
    let broadcast = network | !mask_u32;

    let first_ip = network + 1; // First usable IP (excluding network address)
    let last_ip = broadcast - 1; // Last usable IP (excluding broadcast address)

    let mut ip_list = Vec::new();
    for ip in first_ip..=last_ip {
        ip_list.push(Ipv4Addr::from(ip));
    }

    ip_list
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (tx, rx) = unbounded_channel::<Message>();

    monitor_connections(rx).await;

    loop {
        scan_subnet("192.168.100.1", "255.255.255.0", tx.clone()).await;
        println!("\n");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
