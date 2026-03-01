//! mDNS-SD (multicast DNS Service Discovery) for GMN capture nodes.
//!
//! Registers the capture service on the local network so that gaia-core
//! and the (future) processing container can discover it automatically.

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tracing::{debug, info};

/// How long to scan for existing peers before claiming an instance number.
const DISCOVERY_SCAN: Duration = Duration::from_secs(3);

/// The role a GMN node plays on the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceRole {
    Capture,
    Processing,
    Web,
}

impl ServiceRole {
    /// mDNS service type (≤15 chars per RFC 6763 §7.2).
    pub fn service_type(&self) -> &'static str {
        match self {
            Self::Capture => "_gaia-gmn-cap._tcp.local.",
            Self::Processing => "_gaia-gmn-proc._tcp.local.",
            Self::Web => "_gaia-gmn-web._tcp.local.",
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Capture => "gaia-gmn-capture",
            Self::Processing => "gaia-gmn-processing",
            Self::Web => "gaia-gmn-web",
        }
    }
}

/// Handle returned by [`register`].  Keeps the mDNS daemon alive.
pub struct DiscoveryHandle {
    daemon: ServiceDaemon,
    instance_name: String,
    fullname: String,
}

impl DiscoveryHandle {
    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    pub fn shutdown(self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

/// Register this node on the local network via mDNS.
pub fn register(role: ServiceRole, port: u16) -> Result<DiscoveryHandle> {
    let daemon = ServiceDaemon::new().context("Cannot start mDNS daemon")?;

    // Scan for existing instances of the same role.
    let receiver = daemon
        .browse(role.service_type())
        .context("Cannot browse mDNS")?;

    let mut existing: BTreeSet<u32> = BTreeSet::new();
    let deadline = Instant::now() + DISCOVERY_SCAN;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(n) = parse_instance_number(info.get_fullname(), role.prefix()) {
                    debug!("Found existing {} instance #{}", role.prefix(), n);
                    existing.insert(n);
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = daemon.stop_browse(role.service_type());

    // Pick the next sequential number.
    let our_number = {
        let mut n = 1u32;
        while existing.contains(&n) {
            n += 1;
        }
        n
    };
    let instance_name = format!("{}-{:02}", role.prefix(), our_number);
    let host = format!("{}.local.", instance_name);

    // Explicitly gather non-loopback IP addresses (auto-detect fails in
    // containers).
    let my_addrs: Vec<IpAddr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .map(|iface| iface.ip())
        .collect();

    let addr_str = my_addrs
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let service_info = ServiceInfo::new(
        role.service_type(),
        &instance_name,
        &host,
        addr_str.as_str(),
        port,
        None,
    )
    .context("Cannot create mDNS ServiceInfo")?;

    let fullname = service_info.get_fullname().to_string();
    let registered_addrs = format!("{:?}", service_info.get_addresses());

    daemon
        .register(service_info)
        .context("Cannot register mDNS service")?;

    info!(
        "Registered on mDNS as '{}' (type={}, port={}, addrs={})",
        instance_name,
        role.service_type(),
        port,
        registered_addrs
    );

    Ok(DiscoveryHandle {
        daemon,
        instance_name,
        fullname,
    })
}

fn parse_instance_number(fullname: &str, prefix: &str) -> Option<u32> {
    let instance = fullname.split('.').next()?;
    let suffix = instance.strip_prefix(prefix)?.strip_prefix('-')?;
    suffix.parse().ok()
}
