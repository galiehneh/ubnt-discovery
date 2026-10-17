use std::net::Ipv4Addr;
use std::time::SystemTime;

const SECS_PER_DAY: u32 = 86_400;

/// One MAC/IP pair reported by a device (a device may report several).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceAddress {
    pub mac: String,
    pub ip: Option<Ipv4Addr>,
}

/// Immutable snapshot of a discovered device; updates produce new values.
#[derive(Clone, Debug, PartialEq)]
pub struct DiscoveredDevice {
    pub mac: String,
    pub ip: Option<Ipv4Addr>,
    pub hostname: Option<String>,
    pub essid: Option<String>,
    pub firmware: Option<String>,
    pub model_code: Option<String>,
    pub product_name: Option<String>,
    pub uptime_seconds: Option<u32>,
    pub addresses: Vec<DeviceAddress>,
    pub last_seen: SystemTime,
}

fn non_blank(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|s| !s.trim().is_empty())
}

/// Incoming value when not blank, else the existing one.
fn prefer(incoming: &Option<String>, existing: &Option<String>) -> Option<String> {
    if non_blank(incoming).is_some() {
        incoming.clone()
    } else {
        existing.clone()
    }
}

impl DiscoveredDevice {
    /// Product name, else raw model code, else "Unknown".
    pub fn display_product(&self) -> &str {
        non_blank(&self.product_name)
            .or_else(|| non_blank(&self.model_code))
            .unwrap_or("Unknown")
    }

    pub fn display_ip(&self) -> String {
        self.ip.map(|ip| ip.to_string()).unwrap_or_default()
    }

    /// "HH:MM:SS", "1 day HH:MM:SS" or "N days HH:MM:SS"; empty when unknown.
    pub fn display_uptime(&self) -> String {
        let Some(total) = self.uptime_seconds else {
            return String::new();
        };
        let days = total / SECS_PER_DAY;
        let rem = total % SECS_PER_DAY;
        let clock = format!("{:02}:{:02}:{:02}", rem / 3600, (rem % 3600) / 60, rem % 60);
        match days {
            0 => clock,
            1 => format!("1 day {clock}"),
            n => format!("{n} days {clock}"),
        }
    }

    /// Merge `incoming` into `self` (port of DiscoveryClient.Merge/Prefer).
    pub fn merged_with(&self, incoming: &Self) -> Self {
        let mut addresses: Vec<DeviceAddress> = Vec::new();
        for a in self.addresses.iter().chain(incoming.addresses.iter()) {
            if !addresses.contains(a) {
                addresses.push(a.clone());
            }
        }
        Self {
            mac: self.mac.clone(),
            ip: incoming.ip.or(self.ip),
            hostname: prefer(&incoming.hostname, &self.hostname),
            essid: prefer(&incoming.essid, &self.essid),
            firmware: prefer(&incoming.firmware, &self.firmware),
            model_code: prefer(&incoming.model_code, &self.model_code),
            product_name: prefer(&incoming.product_name, &self.product_name),
            uptime_seconds: incoming.uptime_seconds.or(self.uptime_seconds),
            addresses,
            last_seen: incoming.last_seen,
        }
    }

    /// Case-insensitive substring match across the displayed fields.
    pub fn matches(&self, query: &str) -> bool {
        let haystack = [
            self.display_product(),
            &self.display_ip(),
            &self.mac,
            self.hostname.as_deref().unwrap_or(""),
            self.essid.as_deref().unwrap_or(""),
            self.firmware.as_deref().unwrap_or(""),
            self.model_code.as_deref().unwrap_or(""),
        ]
        .join("\u{1}")
        .to_lowercase();
        haystack.contains(&query.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            last_seen: SystemTime::UNIX_EPOCH,
        }
    }

    fn addr(mac: &str, ip: Option<Ipv4Addr>) -> DeviceAddress {
        DeviceAddress {
            mac: mac.to_string(),
            ip,
        }
    }

    fn uptime(secs: u32) -> String {
        DiscoveredDevice {
            uptime_seconds: Some(secs),
            ..dev("A")
        }
        .display_uptime()
    }

    #[test]
    fn uptime_formats_zero_one_and_many_days() {
        assert_eq!(uptime(0), "00:00:00");
        assert_eq!(uptime(3661), "01:01:01");
        assert_eq!(uptime(86_400), "1 day 00:00:00");
        assert_eq!(uptime(86_400 + 3661), "1 day 01:01:01");
        assert_eq!(uptime(3 * 86_400 + 59), "3 days 00:00:59");
        assert_eq!(uptime(u32::MAX), "49710 days 06:28:15");
        assert_eq!(dev("A").display_uptime(), "");
    }

    #[test]
    fn display_product_falls_back_and_ignores_blank() {
        let mut d = dev("A");
        assert_eq!(d.display_product(), "Unknown");
        d.model_code = Some("N5N".into());
        assert_eq!(d.display_product(), "N5N");
        d.product_name = Some("  ".into());
        assert_eq!(d.display_product(), "N5N");
        d.product_name = Some("NanoStation M5".into());
        assert_eq!(d.display_product(), "NanoStation M5");
    }

    #[test]
    fn display_ip_is_empty_when_missing() {
        let mut d = dev("A");
        assert_eq!(d.display_ip(), "");
        d.ip = Some(Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(d.display_ip(), "10.0.0.1");
    }

    #[test]
    fn merge_prefers_incoming_non_blank_and_keeps_existing_otherwise() {
        let existing = DiscoveredDevice {
            ip: Some(Ipv4Addr::new(10, 0, 0, 1)),
            hostname: Some("old".into()),
            essid: Some("ssid".into()),
            firmware: Some("fw1".into()),
            uptime_seconds: Some(5),
            ..dev("AA")
        };
        let incoming = DiscoveredDevice {
            hostname: Some("new".into()),
            essid: Some("  ".into()),
            firmware: None,
            model_code: Some("N5N".into()),
            uptime_seconds: Some(9),
            last_seen: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(7),
            ..dev("BB")
        };
        let m = existing.merged_with(&incoming);
        assert_eq!(m.mac, "AA");
        assert_eq!(m.ip, Some(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(m.hostname.as_deref(), Some("new"));
        assert_eq!(m.essid.as_deref(), Some("ssid"));
        assert_eq!(m.firmware.as_deref(), Some("fw1"));
        assert_eq!(m.model_code.as_deref(), Some("N5N"));
        assert_eq!(m.uptime_seconds, Some(9));
        assert_eq!(m.last_seen, incoming.last_seen);
    }

    #[test]
    fn merge_takes_incoming_ip_and_dedups_addresses_keeping_first() {
        let a = addr("M1", Some(Ipv4Addr::new(1, 1, 1, 1)));
        let b = addr("M2", None);
        let c = addr("M1", Some(Ipv4Addr::new(2, 2, 2, 2)));
        let existing = DiscoveredDevice {
            ip: Some(Ipv4Addr::new(1, 1, 1, 1)),
            addresses: vec![a.clone(), b.clone()],
            ..dev("AA")
        };
        let incoming = DiscoveredDevice {
            ip: Some(Ipv4Addr::new(2, 2, 2, 2)),
            addresses: vec![a.clone(), c.clone()],
            ..dev("AA")
        };
        let m = existing.merged_with(&incoming);
        assert_eq!(m.ip, Some(Ipv4Addr::new(2, 2, 2, 2)));
        assert_eq!(m.addresses, vec![a, b, c]);
    }

    #[test]
    fn matches_is_case_insensitive_across_fields() {
        let d = DiscoveredDevice {
            ip: Some(Ipv4Addr::new(192, 168, 1, 20)),
            hostname: Some("Rocket-Roof".into()),
            essid: Some("HomeNet".into()),
            firmware: Some("WA.v8.5.5".into()),
            model_code: Some("N5N".into()),
            product_name: Some("NanoStation M5".into()),
            ..dev("00:15:6D:01:02:03")
        };
        for q in [
            "nanostation",
            "192.168.1.2",
            "6d:01",
            "ROCKET",
            "homenet",
            "wa.v8",
            "n5n",
        ] {
            assert!(d.matches(q), "{q}");
        }
        assert!(!d.matches("zzz"));
    }

    #[test]
    fn matches_does_not_span_field_boundaries() {
        let d = DiscoveredDevice {
            hostname: Some("abc".into()),
            essid: Some("def".into()),
            ..dev("AA")
        };
        assert!(!d.matches("abcdef"));
    }
}
