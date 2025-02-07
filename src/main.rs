use tokio::process::Command;

async fn ping(ip: &str) -> bool {
    let output = Command::new("ping")
        .arg("-n")
        .arg("2")
        .arg("-w")
        .arg("1")
        .arg(ip)
        .output()
        .await
        .expect("failed to execute process");

    output.status.success()
}

async fn scan_subnet(subnet: &str, start_ip: u32, end_ip: u32) {
    let mut tasks = Vec::new();

    for i in start_ip..=end_ip {
        let ip = format!("{}.{}", subnet, i);
        let task = tokio::spawn(async move {
            if ping(&ip).await {
                println!("Device found: {}", ip);
            }
        });

        tasks.push(task);
    }

    for join in tasks {
        join.await.unwrap();
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    scan_subnet("192.168.100", 1, 254).await;
}
