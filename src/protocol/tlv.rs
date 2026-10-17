use std::net::Ipv4Addr;
use std::time::SystemTime;

use super::device::{DeviceAddress, DiscoveredDevice};
use super::model_names;

const HEADER_LEN: usize = 4;
const TAG_MAC: u8 = 0x01;
const TAG_MAC_IP: u8 = 0x02;
const TAG_FIRMWARE: u8 = 0x03;
const TAG_UPTIME: u8 = 0x0A;
const TAG_HOSTNAME: u8 = 0x0B;
const TAG_MODEL_CODE: u8 = 0x0C;
const TAG_ESSID: u8 = 0x0D;
const TAG_PRODUCT_NAME: u8 = 0x14;

#[derive(Default)]
struct Fields {
    mac_from_tag: Option<String>,
    firmware: Option<String>,
    hostname: Option<String>,
    model_code: Option<String>,
    essid: Option<String>,
    uptime: Option<u32>,
    addresses: Vec<DeviceAddress>,
}

/// Parse a discovery reply datagram. `None` when too short or no MAC found.
/// Must never panic on malformed or truncated input.
pub fn parse(data: &[u8]) -> Option<DiscoveredDevice> {
    if data.len() < HEADER_LEN {
        return None;
    }
    let mut fields = Fields::default();
    let mut pos = HEADER_LEN;
    while let Some(header) = data.get(pos..pos + 3) {
        let tag = header[0];
        let length = (usize::from(header[1]) << 8) | usize::from(header[2]);
        pos += 3;
        // A bad length ends parsing with whatever we have.
        let Some(value) = data.get(pos..pos.saturating_add(length)) else {
            break;
        };
        pos += length;
        apply_tag(&mut fields, tag, value);
    }
    build_device(fields)
}

fn apply_tag(f: &mut Fields, tag: u8, value: &[u8]) {
    match tag {
        TAG_MAC => {
            if let Some(mac) = value.get(..6) {
                f.mac_from_tag = Some(format_mac(mac));
            }
        }
        TAG_MAC_IP => {
            if let (Some(mac), Some(ip)) = (value.get(..6), value.get(6..10)) {
                f.addresses.push(DeviceAddress {
                    mac: format_mac(mac),
                    ip: Some(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])),
                });
            }
        }
        TAG_FIRMWARE => f.firmware = Some(decode_string(value)),
        TAG_UPTIME => {
            if let Some(b) = value.get(..4) {
                f.uptime = Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
            }
        }
        TAG_HOSTNAME => f.hostname = Some(decode_string(value)),
        TAG_MODEL_CODE => f.model_code = Some(decode_string(value)),
        TAG_ESSID => f.essid = Some(decode_string(value)),
        TAG_PRODUCT_NAME => {
            let product = decode_string(value);
            let code_blank = f.model_code.as_deref().is_none_or(is_blank);
            if !is_blank(&product) && code_blank {
                f.model_code = Some(product);
            }
        }
        _ => {}
    }
}

fn build_device(f: Fields) -> Option<DiscoveredDevice> {
    let mac = f
        .mac_from_tag
        .or_else(|| f.addresses.first().map(|a| a.mac.clone()))
        .filter(|m| !m.is_empty())?;
    let ip = f.addresses.iter().find_map(|a| a.ip);
    let product_name = f.model_code.as_deref().and_then(model_names::resolve);
    Some(DiscoveredDevice {
        mac,
        ip,
        hostname: f.hostname,
        essid: f.essid,
        firmware: f.firmware,
        model_code: f.model_code,
        product_name,
        uptime_seconds: f.uptime,
        addresses: f.addresses,
        last_seen: SystemTime::now(),
    })
}

fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

fn format_mac(mac: &[u8]) -> String {
    mac.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Latin-1 decode, trim trailing NULs then whitespace.
fn decode_string(value: &[u8]) -> String {
    let text: String = value.iter().map(|&b| char::from(b)).collect();
    text.trim_end_matches('\0').trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut r = vec![tag, (value.len() >> 8) as u8, (value.len() & 0xFF) as u8];
        r.extend_from_slice(value);
        r
    }

    fn packet(records: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = vec![0x01, 0x00, 0x00, 0x00];
        for r in records {
            bytes.extend_from_slice(r);
        }
        let body = bytes.len() - 4;
        bytes[2] = (body >> 8) as u8;
        bytes[3] = (body & 0xFF) as u8;
        bytes
    }

    fn concat(a: &[u8], b: &[u8]) -> Vec<u8> {
        [a, b].concat()
    }

    #[test]
    fn extracts_core_fields_from_well_formed_packet() {
        let mac = [0x00, 0x15, 0x6D, 0x01, 0x02, 0x03];
        let p = packet(&[
            tlv(0x02, &concat(&mac, &[192, 168, 1, 20])),
            tlv(0x03, b"WA.v8.5.5"),
            tlv(0x0A, &[0x00, 0x01, 0x00, 0x00]),
            tlv(0x0B, b"rocket-m5"),
            tlv(0x0C, b"N5N"),
            tlv(0x0D, b"my-wlan"),
        ]);
        let d = parse(&p).expect("device");
        assert_eq!(d.mac, "00:15:6D:01:02:03");
        assert_eq!(d.ip, Some(Ipv4Addr::new(192, 168, 1, 20)));
        assert_eq!(d.firmware.as_deref(), Some("WA.v8.5.5"));
        assert_eq!(d.hostname.as_deref(), Some("rocket-m5"));
        assert_eq!(d.model_code.as_deref(), Some("N5N"));
        assert_eq!(d.product_name.as_deref(), Some("NanoStation M5"));
        assert_eq!(d.essid.as_deref(), Some("my-wlan"));
        assert_eq!(d.uptime_seconds, Some(65536));
        assert_eq!(d.addresses.len(), 1);
    }

    #[test]
    fn uses_mac_only_tag_when_no_address_pair() {
        let p = packet(&[
            tlv(0x01, &[0xDC, 0x9F, 0xDB, 0xAA, 0xBB, 0xCC]),
            tlv(0x0C, b"BZ2"),
        ]);
        let d = parse(&p).expect("device");
        assert_eq!(d.mac, "DC:9F:DB:AA:BB:CC");
        assert_eq!(d.product_name.as_deref(), Some("UniFi AP"));
        assert_eq!(d.ip, None);
    }

    #[test]
    fn returns_none_when_packet_too_short() {
        assert!(parse(&[0x01, 0x00]).is_none());
    }

    #[test]
    fn returns_none_when_no_mac_present() {
        assert!(parse(&packet(&[tlv(0x03, b"firmware-only")])).is_none());
    }

    #[test]
    fn keeps_parsed_data_on_truncated_length() {
        let mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let mut p = packet(&[tlv(0x02, &concat(&mac, &[10, 0, 0, 5]))]);
        p.extend_from_slice(&[0x0B, 0xFF, 0xFF, 0x41, 0x42]);
        let d = parse(&p).expect("device");
        assert_eq!(d.mac, "00:11:22:33:44:55");
        assert_eq!(d.hostname, None);
    }

    #[test]
    fn skips_unknown_tags() {
        let p = packet(&[
            tlv(0x7F, &[0xDE, 0xAD, 0xBE, 0xEF]),
            tlv(0x01, &[1, 2, 3, 4, 5, 6]),
        ]);
        assert_eq!(parse(&p).expect("device").mac, "01:02:03:04:05:06");
    }

    #[test]
    fn empty_and_header_only_input_yield_none() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[1, 0, 0, 0]).is_none());
    }

    #[test]
    fn dangling_partial_record_header_is_ignored() {
        let mut p = packet(&[tlv(0x01, &[1, 2, 3, 4, 5, 6])]);
        p.extend_from_slice(&[0x0B, 0x00]);
        assert!(parse(&p).is_some());
    }

    #[test]
    fn short_mac_and_uptime_values_are_ignored() {
        let p = packet(&[tlv(0x01, &[1, 2, 3]), tlv(0x0A, &[0, 0, 1])]);
        assert!(parse(&p).is_none());
        let p = packet(&[tlv(0x02, &[1, 2, 3, 4, 5, 6, 7, 8, 9])]);
        assert!(parse(&p).is_none());
        let p = packet(&[tlv(0x01, &[1, 2, 3, 4, 5, 6]), tlv(0x0A, &[0, 0, 1])]);
        assert_eq!(parse(&p).expect("device").uptime_seconds, None);
    }

    #[test]
    fn strings_trim_nuls_and_whitespace_and_decode_latin1() {
        let p = packet(&[
            tlv(0x01, &[1, 2, 3, 4, 5, 6]),
            tlv(0x0B, b"  host \0\0"),
            tlv(0x0D, &[0x63, 0xE9, 0x00]),
        ]);
        let d = parse(&p).expect("device");
        assert_eq!(d.hostname.as_deref(), Some("host"));
        assert_eq!(d.essid.as_deref(), Some("c\u{e9}"));
    }

    #[test]
    fn product_name_tag_only_fills_blank_model_code() {
        let mac = tlv(0x01, &[1, 2, 3, 4, 5, 6]);
        let p = packet(&[mac.clone(), tlv(0x14, b"Custom Name")]);
        assert_eq!(
            parse(&p).expect("device").model_code.as_deref(),
            Some("Custom Name")
        );
        let p = packet(&[mac.clone(), tlv(0x0C, b"N5N"), tlv(0x14, b"Custom Name")]);
        assert_eq!(
            parse(&p).expect("device").model_code.as_deref(),
            Some("N5N")
        );
        let p = packet(&[mac, tlv(0x0C, b"  "), tlv(0x14, b"Custom Name")]);
        assert_eq!(
            parse(&p).expect("device").model_code.as_deref(),
            Some("Custom Name")
        );
    }

    #[test]
    fn mac_tag_wins_over_address_mac_and_ip_comes_from_pairs() {
        let p = packet(&[
            tlv(0x02, &concat(&[9; 6], &[10, 1, 1, 1])),
            tlv(0x01, &[1, 2, 3, 4, 5, 6]),
        ]);
        let d = parse(&p).expect("device");
        assert_eq!(d.mac, "01:02:03:04:05:06");
        assert_eq!(d.ip, Some(Ipv4Addr::new(10, 1, 1, 1)));
    }

    #[test]
    fn arbitrary_garbage_never_panics() {
        for seed in 0u32..2000 {
            let mut x = seed.wrapping_mul(2654435761).wrapping_add(1);
            let data: Vec<u8> = (0..(seed % 64))
                .map(|_| {
                    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                    (x >> 24) as u8
                })
                .collect();
            let _ = parse(&data);
        }
    }
}
