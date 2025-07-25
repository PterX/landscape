use landscape::{arp::build_gratuitous_arp_packet, iface::get_iface_by_name};
use std::net::Ipv4Addr;
use std::time::Duration;

/// cargo run --package landscape --bin test
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let iface = get_iface_by_name("ens5").await.unwrap();

    let (arp_tx, mut arp_rx) = landscape::arp::create_arp_listen(iface.index).await.unwrap();
    let target_ip = Ipv4Addr::new(10, 10, 10, 112);
    let mac = iface.mac.unwrap();
    tokio::spawn(async move {
        let packet = build_gratuitous_arp_packet(target_ip, mac);
        let mut send_interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            send_interval.tick().await;
            let _ = arp_tx.send(packet.clone()).await;
        }
    });

    while let Some(msg) = arp_rx.recv().await {
        println!("msg: {:?}", msg)
    }
    Ok(())
}
