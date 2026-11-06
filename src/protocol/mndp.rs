//! MikroTik Neighbor Discovery Protocol (MNDP), as used by WinBox's Neighbors tab.
//!
//! Probe: four zero bytes to UDP 5678. Reply: 4-byte header (sequence number)
//! followed by TLVs of `type: u16 BE`, `length: u16 BE`, value.

use std::net::Ipv4Addr;
use std::time::SystemTime;

use super::device::{DeviceAddress, DiscoveredDevice};

pub const PORT: u16 = 5678;
pub const PROBE: [u8; 4] = [0, 0, 0, 0];

const HEADER_LEN: usize = 4;
const TLV_HEADER_LEN: usize = 4;
const T_MAC: u16 = 1;
const T_IDENTITY: u16 = 5;
const T_VERSION: u16 = 7;
const T_PLATFORM: u16 = 8;
const T_UPTIME: u16 = 10;
const T_BOARD: u16 = 12;
const T_IPV4: u16 = 17;

/// Parse an MNDP reply. `None` when too short or no MAC was reported.
/// Never panics on malformed or truncated input.
pub fn parse(data: &[u8]) -> Option<DiscoveredDevice> {
    if data.len() < HEADER_LEN + TLV_HEADER_LEN {
        return None;
    }
    let mut mac = None;
    let mut ip = None;
    let (mut identity, mut version, mut platform, mut board) = (None, None, None, None);
    let mut uptime = None;
    let mut pos = HEADER_LEN;
    while let Some(h) = data.get(pos..pos + TLV_HEADER_LEN) {
        let tag = u16::from_be_bytes([h[0], h[1]]);
        let len = usize::from(u16::from_be_bytes([h[2], h[3]]));
        pos += TLV_HEADER_LEN;
        let Some(v) = data.get(pos..pos + len) else {
            break;
        };
        pos += len;
        match tag {
            T_MAC => mac = v.get(..6).map(format_mac),
            T_IDENTITY => identity = Some(text(v)),
            T_VERSION => version = Some(text(v)),
            T_PLATFORM => platform = Some(text(v)),
            T_BOARD => board = Some(text(v)),
            // Uptime is little-endian, unlike the framing.
            T_UPTIME => uptime = v.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            T_IPV4 => ip = v.get(..4).map(|b| Ipv4Addr::new(b[0], b[1], b[2], b[3])),
            _ => {}
        }
    }
    let mac = mac?;
    Some(DiscoveredDevice {
        addresses: vec![DeviceAddress {
            mac: mac.clone(),
            ip,
        }],
        mac,
        ip,
        hostname: identity,
        essid: platform,
        firmware: version,
        model_code: board.clone(),
        product_name: board,
        uptime_seconds: uptime,
        web_port: None,
        last_seen: SystemTime::now(),
    })
}

fn format_mac(mac: &[u8]) -> String {
    mac.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn text(v: &[u8]) -> String {
    String::from_utf8_lossy(v)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tlv(tag: u16, v: &[u8]) -> Vec<u8> {
        let mut out = tag.to_be_bytes().to_vec();
        out.extend((v.len() as u16).to_be_bytes());
        out.extend_from_slice(v);
        out
    }

    fn packet() -> Vec<u8> {
        let mut p = vec![0, 1, 0, 0];
        p.extend(tlv(T_MAC, &[0x64, 0xD1, 0x54, 0x01, 0x02, 0x03]));
        p.extend(tlv(T_IDENTITY, b"core-router"));
        p.extend(tlv(T_VERSION, b"7.14 (stable)"));
        p.extend(tlv(T_PLATFORM, b"MikroTik"));
        p.extend(tlv(T_UPTIME, &3661u32.to_le_bytes()));
        p.extend(tlv(T_BOARD, b"RB4011iGS+"));
        p.extend(tlv(T_IPV4, &[192, 168, 88, 1]));
        p
    }

    #[test]
    fn parses_all_fields() {
        let d = parse(&packet()).expect("device");
        assert_eq!(d.mac, "64:D1:54:01:02:03");
        assert_eq!(d.ip, Some(Ipv4Addr::new(192, 168, 88, 1)));
        assert_eq!(d.hostname.as_deref(), Some("core-router"));
        assert_eq!(d.firmware.as_deref(), Some("7.14 (stable)"));
        assert_eq!(d.essid.as_deref(), Some("MikroTik"));
        assert_eq!(d.display_product(), "RB4011iGS+");
        assert_eq!(d.uptime_seconds, Some(3661));
    }

    #[test]
    fn rejects_short_and_macless_packets() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 0, 0, 0]).is_none());
        let mut p = vec![0, 0, 0, 0];
        p.extend(tlv(T_IDENTITY, b"x"));
        assert!(parse(&p).is_none());
    }

    #[test]
    fn truncated_value_keeps_earlier_fields() {
        let mut p = packet();
        p.truncate(p.len() - 2);
        let d = parse(&p).expect("device");
        assert_eq!(d.hostname.as_deref(), Some("core-router"));
        assert_eq!(d.ip, None);
    }

    #[test]
    fn never_panics_on_every_prefix() {
        let p = packet();
        for n in 0..p.len() {
            let _ = parse(&p[..n]);
        }
    }
}
