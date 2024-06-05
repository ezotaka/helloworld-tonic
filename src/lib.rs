use std::net::IpAddr;
use tokio::net::lookup_host;

pub async fn get_local_ip() -> Result<IpAddr, Box<dyn std::error::Error>> {
    let hostname = hostname::get()?;
    let hostname = hostname
        .to_str()
        .ok_or("Failed to convert hostname to str")?;

    let addrs_iter = lookup_host((hostname, 0)).await?;
    for addr in addrs_iter {
        if let IpAddr::V4(ipv4_addr) = addr.ip() {
            return Ok(IpAddr::V4(ipv4_addr));
        }
    }

    Err("No IPv4 address found".into())
}
