//! QNAP Qfinder discovery.
//!
//! Probe: 16-byte datagram broadcast to UDP 8097. A reply is a 16-byte header
//! (NAS MAC, the marker `Se`, flags) followed by records:
//!
//! * normal record: `tag: u8`, `len: u8`, `len` bytes of data
//! * extended record: tag `0xCA`, `len: u8`, then `tag: u32 LE`, `len2: u8`, data
//!
//! Layout derived from public Qfinder captures and from replies of current
//! QTS/QuTS hero devices. Unknown records are ignored.

use std::net::Ipv4Addr;
use std::time::SystemTime;

use super::device::{DeviceAddress, DiscoveredDevice};

pub const PORT: u16 = 8097;
/// Default QTS web UI port, used when the reply carries none.
pub const WEB_PORT: u16 = 8080;

/// Brief ("query") and detailed probes as sent by Qfinder.
pub const PROBES: [[u8; 16]; 2] = [
    [0, 0, 0, 0, 0, 0, 0x43, 0x6C, 0, 0, 0x01, 0, 0, 0, 0x25, 0x04],
    [
        0x00, 0x08, 0x9B, 0xC3, 0xEC, 0x6A, 0x43, 0x6C, 0x02, 0, 0x05, 0, 0, 0, 0x25, 0x04,
    ],
];

const MAC_LEN: usize = 6;
const MARKER: [u8; 2] = [0x53, 0x65]; // "Se"
const HEADER_LEN: usize = 16;
const EXTENDED: u8 = 0xCA;

const T_HOSTNAME: u32 = 0x01;
const T_IPV4: u32 = 0x05;
const T_KIND: u32 = 0x09;
const T_MAC: u32 = 0x0C;
const T_MODEL: u32 = 0x28;
const T_MODEL_SHORT: u32 = 0x29;
const T_BUILD_DATE: u32 = 0x2D;
const T_WEB_PORT: u32 = 0x35;
const T_MODEL_DISPLAY: u32 = 0x49;
/// Extended record (0xCA, id 0x26E): firmware build number.
const T_BUILD_NUMBER: u32 = 0x100 | 0x26E;

/// Walk the records after the header; stops at the first malformed one.
fn records(data: &[u8]) -> Vec<(u32, &[u8])> {
    let mut out = Vec::new();
    let mut pos = HEADER_LEN;
    while let Some(&tag) = data.get(pos) {
        let Some(&len) = data.get(pos + 1) else { break };
        let Some(body) = data.get(pos + 2..pos + 2 + usize::from(len)) else {
            break;
        };
        pos += 2 + usize::from(len);
        if tag == EXTENDED {
            let (Some(id), Some(&len2)) = (body.get(..4), body.get(4)) else {
                continue;
            };
            let id = u32::from_le_bytes([id[0], id[1], id[2], id[3]]) | 0x100;
            if let Some(value) = body.get(5..5 + usize::from(len2)) {
                out.push((id, value));
            }
        } else {
            out.push((u32::from(tag), body));
        }
    }
    out
}

fn text(v: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(v);
    let s = s.trim_end_matches('\0').trim();
    (!s.is_empty() && s.chars().all(|c| !c.is_control())).then(|| s.to_string())
}

fn format_mac(mac: &[u8]) -> String {
    mac.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

pub fn parse(data: &[u8]) -> Option<DiscoveredDevice> {
    if data.len() < HEADER_LEN || data.get(MAC_LEN..MAC_LEN + 2)? != MARKER {
        return None;
    }
    let mut mac = Some(format_mac(&data[..MAC_LEN])).filter(|_| data[..MAC_LEN] != [0; MAC_LEN]);
    let (mut hostname, mut kind, mut model, mut ip) = (None, None, None, None);
    let (mut build_date, mut build_number, mut web_port) = (None, None, None);
    for (tag, v) in records(data) {
        match tag {
            T_HOSTNAME => hostname = text(v),
            T_KIND => kind = text(v),
            T_MODEL_DISPLAY => model = text(v).or(model),
            T_MODEL | T_MODEL_SHORT => model = model.or_else(|| text(v)),
            T_BUILD_DATE => build_date = text(v),
            T_BUILD_NUMBER => build_number = text(v),
            T_IPV4 => ip = v.get(..4).map(|b| Ipv4Addr::new(b[0], b[1], b[2], b[3])),
            T_MAC => {
                if let Some(m) = v.get(..MAC_LEN).filter(|m| **m != [0; MAC_LEN]) {
                    mac = Some(format_mac(m));
                }
            }
            T_WEB_PORT => {
                web_port = v
                    .get(..4)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                    .and_then(|p| u16::try_from(p).ok())
                    .filter(|p| *p != 0);
            }
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
        hostname,
        essid: kind,
        // The broadcast reply carries no dotted version, only date and build.
        firmware: match (build_date, build_number) {
            (Some(d), Some(n)) => Some(format!("{d} (build {n})")),
            (d, n) => d.or(n),
        },
        model_code: model.clone(),
        product_name: model,
        uptime_seconds: None,
        web_port,
        last_seen: SystemTime::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(tag: u8, v: &[u8]) -> Vec<u8> {
        let mut out = vec![tag, v.len() as u8];
        out.extend_from_slice(v);
        out
    }

    fn ext(id: u32, v: &[u8]) -> Vec<u8> {
        let mut body = id.to_le_bytes().to_vec();
        body.push(v.len() as u8);
        body.extend_from_slice(v);
        rec(EXTENDED, &body)
    }

    fn reply() -> Vec<u8> {
        let mut p = vec![0x24, 0x5E, 0xBE, 0x7B, 0x0F, 0xAF, 0x53, 0x65];
        p.extend([1, 0, 1, 0, 0, 0, 0x25, 0x04]);
        p.extend(ext(0x1C6, b"192.168.120.178\0"));
        p.extend(rec(0x01, b"NAS432NC01\0"));
        p.extend(rec(0x09, b"NAS\0"));
        p.extend(rec(0x28, b"TS-X83XU\0"));
        p.extend(rec(0x49, b"TS-2483XU-RP\0"));
        p.extend(rec(0x2D, b"20261014\0"));
        p.extend(ext(0x26E, b"3652\0"));
        p.extend(rec(0x35, &8080u32.to_le_bytes()));
        p.extend(rec(0x05, &[172, 16, 1, 151]));
        p.extend(rec(0x0C, &[0x24, 0x5E, 0xBE, 0x7B, 0x0F, 0xAF]));
        p
    }

    #[test]
    fn probes_are_sixteen_bytes() {
        assert!(PROBES.iter().all(|p| p.len() == 16));
    }

    #[test]
    fn parses_records() {
        let d = parse(&reply()).expect("device");
        assert_eq!(d.mac, "24:5E:BE:7B:0F:AF");
        assert_eq!(d.hostname.as_deref(), Some("NAS432NC01"));
        assert_eq!(d.display_product(), "TS-2483XU-RP");
        assert_eq!(d.ip, Some(Ipv4Addr::new(172, 16, 1, 151)));
        assert_eq!(d.essid.as_deref(), Some("NAS"));
        assert_eq!(d.firmware.as_deref(), Some("20261014 (build 3652)"));
        assert_eq!(d.web_port, Some(8080));
    }

    #[test]
    fn rejects_non_qnap_and_short_data() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[1; 40]).is_none());
        let mut zero_mac = reply();
        zero_mac[..6].fill(0);
        let mut no_records = zero_mac[..HEADER_LEN].to_vec();
        assert!(parse(&no_records).is_none());
        no_records.extend(rec(0x01, b"x\0"));
        assert!(parse(&no_records).is_none());
    }

    #[test]
    fn never_panics_on_every_prefix() {
        let p = reply();
        for n in 0..p.len() {
            let _ = parse(&p[..n]);
        }
    }
}
