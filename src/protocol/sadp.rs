//! Hikvision SADP (Search Active Devices Protocol).
//!
//! Probe: XML `<Probe>` multicast to 239.255.255.250:37020. Devices answer with
//! an XML `<ProbeMatch>` document (to the same group or unicast).

use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};

use super::device::{DeviceAddress, DiscoveredDevice};

pub const PORT: u16 = 37020;
pub const GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// Inquiry probe with a fresh GUID-shaped `Uuid`.
pub fn probe() -> Vec<u8> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let h = nanos.to_be_bytes();
    let uuid = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8], h[9], h[10], h[11], h[12], h[13],
        h[14], h[15]
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><Probe><Uuid>{uuid}</Uuid><Types>inquiry</Types></Probe>"
    )
    .into_bytes()
}

/// Text of the first `<name>…</name>` element, trimmed; `None` when absent/empty.
fn element<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = xml.find(&open)? + open.len();
    let end = start + xml[start..].find(&close)?;
    let value = xml[start..end].trim();
    (!value.is_empty()).then_some(value)
}

fn owned(xml: &str, name: &str) -> Option<String> {
    element(xml, name).map(decode_entities)
}

fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Normalize `aa-bb-cc-…`/`aa:bb:…` to upper-case colon form.
fn normalize_mac(raw: &str) -> Option<String> {
    let hex: String = raw.chars().filter(char::is_ascii_hexdigit).collect();
    if hex.len() != 12 {
        return None;
    }
    let pairs: Vec<&str> = (0..6).map(|i| &hex[i * 2..i * 2 + 2]).collect();
    Some(pairs.join(":").to_uppercase())
}

/// Parse a `<ProbeMatch>` reply; the probe echo and foreign XML give `None`.
pub fn parse(data: &[u8]) -> Option<DiscoveredDevice> {
    let xml = std::str::from_utf8(data).ok()?;
    if !xml.contains("<ProbeMatch>") {
        return None;
    }
    let mac = normalize_mac(element(xml, "MAC")?)?;
    let ip = element(xml, "IPv4Address").and_then(|s| s.parse::<Ipv4Addr>().ok());
    // Real devices often put a numeric type code in <DeviceType>; the readable
    // model is in <DeviceDescription>.
    let model = owned(xml, "DeviceDescription")
        .or_else(|| owned(xml, "DeviceType").filter(|t| !t.chars().all(|c| c.is_ascii_digit())));
    let status = element(xml, "Activated").map(|a| {
        if a.eq_ignore_ascii_case("true") {
            "Active"
        } else {
            "Inactive"
        }
    });
    Some(DiscoveredDevice {
        addresses: vec![DeviceAddress {
            mac: mac.clone(),
            ip,
        }],
        mac,
        ip,
        hostname: owned(xml, "DeviceSN"),
        essid: status.map(str::to_string),
        firmware: owned(xml, "SoftwareVersion"),
        model_code: model.clone(),
        product_name: model,
        uptime_seconds: None,
        web_port: element(xml, "HttpPort").and_then(|p| p.parse().ok()),
        last_seen: SystemTime::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLY: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ProbeMatch><Uuid>X</Uuid>\
        <Types>inquiry</Types><DeviceType>DS-2CD2143G0-I</DeviceType>\
        <DeviceDescription>DS-2CD2143G0-I</DeviceDescription><DeviceSN>DS-2CD2143G0-I2020</DeviceSN>\
        <CommandPort>8000</CommandPort><HttpPort>80</HttpPort><MAC>c0-56-e3-0a-0b-0c</MAC>\
        <IPv4Address>192.168.1.64</IPv4Address><IPv4SubnetMask>255.255.255.0</IPv4SubnetMask>\
        <SoftwareVersion>V5.5.82build 190909</SoftwareVersion><Activated>true</Activated></ProbeMatch>";

    #[test]
    fn probe_is_well_formed() {
        let p = String::from_utf8(probe()).unwrap();
        assert!(p.contains("<Types>inquiry</Types>") && p.contains("<Uuid>"));
        assert!(p.ends_with("</Probe>"));
    }

    #[test]
    fn parses_reply() {
        let d = parse(REPLY.as_bytes()).expect("device");
        assert_eq!(d.mac, "C0:56:E3:0A:0B:0C");
        assert_eq!(d.ip, Some(Ipv4Addr::new(192, 168, 1, 64)));
        assert_eq!(d.display_product(), "DS-2CD2143G0-I");
        assert_eq!(d.hostname.as_deref(), Some("DS-2CD2143G0-I2020"));
        assert_eq!(d.essid.as_deref(), Some("Active"));
        assert_eq!(d.firmware.as_deref(), Some("V5.5.82build 190909"));
        assert_eq!(d.web_port, Some(80));
    }

    #[test]
    fn inactive_device_is_reported() {
        let xml = REPLY.replace("<Activated>true", "<Activated>false");
        assert_eq!(parse(xml.as_bytes()).unwrap().essid.as_deref(), Some("Inactive"));
    }

    #[test]
    fn ignores_probe_echo_garbage_and_missing_mac() {
        assert!(parse(&probe()).is_none());
        assert!(parse(&[0xFF, 0xFE, 0x00]).is_none());
        assert!(parse(b"<ProbeMatch><MAC>zz</MAC></ProbeMatch>").is_none());
        assert!(parse(b"<ProbeMatch></ProbeMatch>").is_none());
    }
}
