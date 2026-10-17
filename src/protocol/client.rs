use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};

use super::device::DiscoveredDevice;
use super::{request, tlv};

const RECV_TIMEOUT: Duration = Duration::from_millis(200);
const RESEND_INTERVAL: Duration = Duration::from_secs(1);
const POLL_SLICE: Duration = Duration::from_millis(100);
const RECV_ERROR_BACKOFF: Duration = Duration::from_millis(10);
const RECV_BUF_LEN: usize = 4096;

/// A bound sender socket plus its subnet-directed broadcast address.
struct Sender {
    socket: UdpSocket,
    subnet_broadcast: Ipv4Addr,
}

/// Thread-safe MAC-keyed device table.
#[derive(Default)]
struct DeviceTable {
    devices: Mutex<HashMap<String, DiscoveredDevice>>,
}

impl DeviceTable {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, DiscoveredDevice>> {
        // A poisoned lock still holds consistent data (we only insert whole values).
        self.devices
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Insert or merge `incoming`. Returns the new snapshot only when something
    /// other than `last_seen` changed, so repeat replies don't flood the UI.
    fn merge(&self, incoming: DiscoveredDevice) -> Option<DiscoveredDevice> {
        let key = incoming.mac.to_uppercase();
        let mut map = self.lock();
        let (merged, changed) = match map.get(&key) {
            Some(existing) => {
                let merged = existing.merged_with(&incoming);
                let unchanged = DiscoveredDevice {
                    last_seen: existing.last_seen,
                    ..merged.clone()
                } == *existing;
                (merged, !unchanged)
            }
            None => (incoming, true),
        };
        map.insert(key, merged.clone());
        changed.then_some(merged)
    }
}

/// Subnet-directed broadcast address: `ip | !netmask`.
fn subnet_broadcast(ip: Ipv4Addr, netmask: Ipv4Addr) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(ip) | !u32::from(netmask))
}

/// Run one scan, blocking until `duration` elapses or `cancel` is set.
/// `on_device` is called from background threads with the merged snapshot
/// whenever a device is found or updated.
pub fn scan<F>(duration: Duration, cancel: Arc<AtomicBool>, on_device: F)
where
    F: Fn(DiscoveredDevice) + Send + Sync + 'static,
{
    let deadline = Instant::now() + duration;
    let senders = build_senders();
    let listener = create_port_listener();
    let table = DeviceTable::default();
    let cancel = cancel.as_ref();
    let on_device = &on_device;
    let table = &table;
    let senders = &senders;

    thread::scope(|scope| {
        for s in senders {
            if let Ok(clone) = s.socket.try_clone() {
                scope.spawn(move || receive_loop(clone, cancel, deadline, table, on_device));
            }
        }
        if let Some(l) = listener {
            scope.spawn(move || receive_loop(l, cancel, deadline, table, on_device));
        }
        scope.spawn(move || send_loop(senders, cancel, deadline));
    });
}

fn should_stop(cancel: &AtomicBool, deadline: Instant) -> bool {
    cancel.load(Ordering::Relaxed) || Instant::now() >= deadline
}

fn build_senders() -> Vec<Sender> {
    let Ok(interfaces) = if_addrs::get_if_addrs() else {
        return Vec::new();
    };
    interfaces
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.addr {
            if_addrs::IfAddr::V4(v4) => Some((v4.ip, v4.netmask)),
            if_addrs::IfAddr::V6(_) => None,
        })
        .filter_map(|(ip, mask)| {
            let socket = bind_socket(SocketAddrV4::new(ip, 0)).ok()?;
            Some(Sender {
                socket,
                subnet_broadcast: subnet_broadcast(ip, mask),
            })
        })
        .collect()
}

fn create_port_listener() -> Option<UdpSocket> {
    bind_socket(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, request::PORT)).ok()
}

fn bind_socket(addr: SocketAddrV4) -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_broadcast(true)?;
    socket.bind(&SocketAddr::V4(addr).into())?;
    let socket: UdpSocket = socket.into();
    socket.set_read_timeout(Some(RECV_TIMEOUT))?;
    Ok(socket)
}

fn send_loop(senders: &[Sender], cancel: &AtomicBool, deadline: Instant) {
    let global = SocketAddrV4::new(Ipv4Addr::BROADCAST, request::PORT);
    while !should_stop(cancel, deadline) {
        for s in senders {
            let subnet = SocketAddrV4::new(s.subnet_broadcast, request::PORT);
            for probe in request::ALL {
                // Per-target send failures are expected on some adapters.
                let _ = s.socket.send_to(probe, global);
                let _ = s.socket.send_to(probe, subnet);
            }
        }
        sleep_unless_stopped(RESEND_INTERVAL, cancel, deadline);
    }
}

fn sleep_unless_stopped(total: Duration, cancel: &AtomicBool, deadline: Instant) {
    let end = Instant::now() + total;
    while Instant::now() < end && !should_stop(cancel, deadline) {
        thread::sleep(POLL_SLICE);
    }
}

fn receive_loop<F>(
    socket: UdpSocket,
    cancel: &AtomicBool,
    deadline: Instant,
    table: &DeviceTable,
    on_device: &F,
) where
    F: Fn(DiscoveredDevice),
{
    let mut buf = [0u8; RECV_BUF_LEN];
    while !should_stop(cancel, deadline) {
        match socket.recv_from(&mut buf) {
            Ok((len, from)) => {
                if let Some(device) = buf.get(..len).and_then(|d| parse_reply(d, from.ip())) {
                    if let Some(updated) = table.merge(device) {
                        on_device(updated);
                    }
                }
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            // Transient error (e.g. ICMP-induced reset); keep listening.
            Err(_) => thread::sleep(RECV_ERROR_BACKOFF),
        }
    }
}

/// Parse a datagram, filling in the sender IPv4 when the packet carried none.
fn parse_reply(data: &[u8], sender: IpAddr) -> Option<DiscoveredDevice> {
    let mut device = tlv::parse(data)?;
    if device.ip.is_none() {
        if let IpAddr::V4(v4) = sender {
            device.ip = Some(v4);
        }
    }
    Some(device)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::DeviceAddress;
    use std::net::Ipv6Addr;
    use std::time::SystemTime;

    fn dev(mac: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            mac: mac.to_string(),
            ip: None,
            hostname: None,
            essid: None,
            firmware: None,
            model_code: None,
            product_name: None,
            uptime_seconds: None,
            addresses: Vec::new(),
            last_seen: SystemTime::now(),
        }
    }

    #[test]
    fn computes_subnet_broadcast() {
        let b = |ip: [u8; 4], m: [u8; 4]| subnet_broadcast(ip.into(), m.into());
        assert_eq!(
            b([192, 168, 1, 20], [255, 255, 255, 0]),
            Ipv4Addr::new(192, 168, 1, 255)
        );
        assert_eq!(
            b([10, 1, 2, 3], [255, 0, 0, 0]),
            Ipv4Addr::new(10, 255, 255, 255)
        );
        assert_eq!(
            b([172, 16, 5, 130], [255, 255, 255, 128]),
            Ipv4Addr::new(172, 16, 5, 255)
        );
        assert_eq!(
            b([1, 2, 3, 4], [255, 255, 255, 255]),
            Ipv4Addr::new(1, 2, 3, 4)
        );
    }

    #[test]
    fn table_inserts_then_merges_case_insensitively() {
        let table = DeviceTable::default();
        let first = DiscoveredDevice {
            hostname: Some("h".into()),
            ..dev("aa:bb")
        };
        let inserted = table.merge(first).expect("new device is reported");
        assert_eq!(inserted.hostname.as_deref(), Some("h"));
        let second = DiscoveredDevice {
            essid: Some("e".into()),
            addresses: vec![DeviceAddress {
                mac: "AA:BB".into(),
                ip: None,
            }],
            ..dev("AA:BB")
        };
        let merged = table.merge(second).expect("changed device is reported");
        assert_eq!(merged.hostname.as_deref(), Some("h"));
        assert_eq!(merged.essid.as_deref(), Some("e"));
        assert_eq!(table.lock().len(), 1);
    }

    #[test]
    fn table_survives_poisoned_lock() {
        let table = Arc::new(DeviceTable::default());
        let t = Arc::clone(&table);
        let _ = thread::spawn(move || {
            let _guard = t.lock();
            panic!("poison");
        })
        .join();
        assert_eq!(table.merge(dev("AA")).expect("new").mac, "AA");
    }

    #[test]
    fn table_skips_replies_that_only_change_last_seen() {
        let table = DeviceTable::default();
        let first = DiscoveredDevice {
            hostname: Some("h".into()),
            ..dev("AA")
        };
        assert!(table.merge(first.clone()).is_some());
        let repeat = DiscoveredDevice {
            last_seen: first.last_seen + Duration::from_secs(1),
            ..first
        };
        assert!(table.merge(repeat).is_none());
    }

    fn sample_packet(with_ip: bool) -> Vec<u8> {
        let mut p = vec![1, 0, 0, 0, 0x01, 0, 6, 1, 2, 3, 4, 5, 6];
        if with_ip {
            p.extend_from_slice(&[0x02, 0, 10, 1, 2, 3, 4, 5, 6, 9, 9, 9, 9]);
        }
        p
    }

    #[test]
    fn parse_reply_fills_sender_ip_when_missing() {
        let sender = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));
        let d = parse_reply(&sample_packet(false), sender).expect("device");
        assert_eq!(d.ip, Some(Ipv4Addr::new(10, 0, 0, 7)));
    }

    #[test]
    fn parse_reply_keeps_packet_ip_and_ignores_v6_sender() {
        let sender = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));
        let d = parse_reply(&sample_packet(true), sender).expect("device");
        assert_eq!(d.ip, Some(Ipv4Addr::new(9, 9, 9, 9)));
        let v6 = IpAddr::V6(Ipv6Addr::LOCALHOST);
        assert_eq!(
            parse_reply(&sample_packet(false), v6).expect("device").ip,
            None
        );
        assert!(parse_reply(&[1, 2], sender).is_none());
    }

    #[test]
    fn scan_returns_promptly_when_cancelled() {
        let cancel = Arc::new(AtomicBool::new(true));
        let start = Instant::now();
        scan(Duration::from_secs(30), cancel, |_| {});
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
