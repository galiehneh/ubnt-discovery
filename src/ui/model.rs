//! Pure, GUI-free logic shared by the windows: row formatting, sorting,
//! filtering, clipboard text and the scan countdown label.

use std::cmp::Ordering;

use ubnt_discovery::protocol::{DeviceAddress, DiscoveredDevice};

/// Length of one scan in seconds.
pub const SCAN_SECONDS: u64 = 6;

/// Column headers and initial widths (logical pixels) of the main list.
pub const COLUMNS: [(&str, i32); 6] = [
    ("Product Name", 160),
    ("IP Address", 110),
    ("Hardware Address", 130),
    ("System Name", 140),
    ("SSID", 110),
    ("Firmware Version", 170),
];

const IP_COLUMN: usize = 1;

/// Active sort order; the default is product name ascending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortSpec {
    pub column: usize,
    pub ascending: bool,
}

impl SortSpec {
    pub const DEFAULT: SortSpec = SortSpec {
        column: 0,
        ascending: true,
    };

    /// Order after the user clicks the header of `column`.
    pub fn toggled(self, column: usize) -> SortSpec {
        SortSpec {
            column,
            ascending: self.column != column || !self.ascending,
        }
    }
}

/// The six displayed cells of a device, in column order.
pub fn row_cells(device: &DiscoveredDevice) -> [String; 6] {
    [
        device.display_product().to_string(),
        device.display_ip(),
        device.mac.clone(),
        device.hostname.clone().unwrap_or_default(),
        device.essid.clone().unwrap_or_default(),
        device.firmware.clone().unwrap_or_default(),
    ]
}

/// Compare two devices by one column (case-insensitive; IPs numerically).
pub fn compare_by_column(a: &DiscoveredDevice, b: &DiscoveredDevice, column: usize) -> Ordering {
    if column == IP_COLUMN {
        return a.ip.cmp(&b.ip);
    }
    let column = column.min(COLUMNS.len() - 1);
    let (ca, cb) = (row_cells(a), row_cells(b));
    ca[column].to_lowercase().cmp(&cb[column].to_lowercase())
}

/// Filter by `query` (trimmed; empty = all) and sort. Ties break on MAC so the
/// order is stable regardless of hash-map iteration order.
pub fn filter_and_sort<'a>(
    devices: impl IntoIterator<Item = &'a DiscoveredDevice>,
    query: &str,
    sort: SortSpec,
) -> Vec<DiscoveredDevice> {
    let query = query.trim();
    let mut rows: Vec<DiscoveredDevice> = devices
        .into_iter()
        .filter(|d| query.is_empty() || d.matches(query))
        .cloned()
        .collect();
    rows.sort_by(|a, b| {
        let primary = compare_by_column(a, b, sort.column);
        let primary = if sort.ascending {
            primary
        } else {
            primary.reverse()
        };
        primary.then_with(|| a.mac.cmp(&b.mac))
    });
    rows
}

/// Tab-separated lines (CRLF terminated) for the clipboard.
pub fn build_tsv(rows: &[[String; 6]]) -> String {
    rows.iter().map(|r| r.join("\t") + "\r\n").collect()
}

/// Text of the scan button while scanning.
pub fn countdown_text(elapsed_secs: u64) -> String {
    format!("Scanning ({}s)", SCAN_SECONDS.saturating_sub(elapsed_secs))
}

/// Text of the scan button while waiting for the automatic first scan.
pub fn autostart_text(remaining_secs: u64) -> String {
    format!("Scan in {remaining_secs}s")
}

/// Status-bar text.
pub fn total_text(count: usize) -> String {
    format!("Total: {count}")
}

/// Rows for the details address list: reported addresses, or the primary pair.
pub fn address_rows(device: &DiscoveredDevice) -> Vec<(String, String)> {
    let primary = [DeviceAddress {
        mac: device.mac.clone(),
        ip: device.ip,
    }];
    let addresses: &[DeviceAddress] = if device.addresses.is_empty() {
        &primary
    } else {
        &device.addresses
    };
    addresses
        .iter()
        .map(|a| {
            (
                a.mac.clone(),
                a.ip.map(|i| i.to_string()).unwrap_or_default(),
            )
        })
        .collect()
}

/// Value shown in the details dialog for an optional text field.
pub fn or_unknown(value: Option<&str>) -> String {
    match value {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::SystemTime;

    fn device(mac: &str, ip: Option<[u8; 4]>, name: Option<&str>) -> DiscoveredDevice {
        DiscoveredDevice {
            mac: mac.to_string(),
            ip: ip.map(Ipv4Addr::from),
            hostname: None,
            essid: None,
            firmware: None,
            model_code: None,
            product_name: name.map(str::to_string),
            uptime_seconds: None,
            addresses: Vec::new(),
            last_seen: SystemTime::UNIX_EPOCH,
        }
    }

    fn macs(rows: &[DiscoveredDevice]) -> Vec<&str> {
        rows.iter().map(|d| d.mac.as_str()).collect()
    }

    #[test]
    fn toggled_flips_same_column_and_resets_on_new_column() {
        let s = SortSpec::DEFAULT.toggled(0);
        assert!(!s.ascending);
        assert!(s.toggled(0).ascending);
        let s = s.toggled(2);
        assert_eq!(
            s,
            SortSpec {
                column: 2,
                ascending: true
            }
        );
    }

    #[test]
    fn tsv_joins_cells_with_tabs_and_crlf() {
        let rows = [
            ["a", "b", "c", "d", "e", "f"].map(String::from),
            ["1", "", "3", "", "", "6"].map(String::from),
        ];
        assert_eq!(build_tsv(&rows), "a\tb\tc\td\te\tf\r\n1\t\t3\t\t\t6\r\n");
        assert_eq!(build_tsv(&[]), "");
    }

    #[test]
    fn countdown_never_goes_negative() {
        assert_eq!(countdown_text(0), "Scanning (6s)");
        assert_eq!(countdown_text(5), "Scanning (1s)");
        assert_eq!(countdown_text(99), "Scanning (0s)");
    }

    #[test]
    fn autostart_text_shows_remaining_seconds() {
        assert_eq!(autostart_text(4), "Scan in 4s");
        assert_eq!(autostart_text(1), "Scan in 1s");
    }

    #[test]
    fn total_text_formats_count() {
        assert_eq!(total_text(0), "Total: 0");
        assert_eq!(total_text(12), "Total: 12");
    }

    #[test]
    fn ip_column_sorts_numerically() {
        let a = device("A", Some([10, 0, 0, 9]), None);
        let b = device("B", Some([10, 0, 0, 10]), None);
        assert_eq!(compare_by_column(&a, &b, 1), Ordering::Less);
    }

    #[test]
    fn sort_is_case_insensitive_and_reversible() {
        let devs = [
            device("02", None, Some("beta")),
            device("01", None, Some("Alpha")),
            device("03", None, Some("gamma")),
        ];
        let asc = filter_and_sort(&devs, "", SortSpec::DEFAULT);
        assert_eq!(macs(&asc), ["01", "02", "03"]);
        let spec = SortSpec {
            column: 0,
            ascending: false,
        };
        let desc = filter_and_sort(&devs, "  ", spec);
        assert_eq!(macs(&desc), ["03", "02", "01"]);
    }

    #[test]
    fn filter_applies_query() {
        let devs = [
            device("AA", None, Some("NanoStation")),
            device("BB", None, Some("Router")),
        ];
        let rows = filter_and_sort(&devs, "nano", SortSpec::DEFAULT);
        assert_eq!(macs(&rows), ["AA"]);
    }

    #[test]
    fn address_rows_fall_back_to_primary() {
        let d = device("AA:BB", Some([1, 2, 3, 4]), None);
        assert_eq!(
            address_rows(&d),
            [("AA:BB".to_string(), "1.2.3.4".to_string())]
        );
        let mut d2 = d.clone();
        d2.addresses = vec![DeviceAddress {
            mac: "CC".into(),
            ip: None,
        }];
        assert_eq!(address_rows(&d2), [("CC".to_string(), String::new())]);
    }

    #[test]
    fn row_cells_use_empty_strings_for_missing_fields() {
        let d = device("AA", Some([1, 1, 1, 1]), Some("X"));
        let cells = row_cells(&d);
        assert_eq!(cells[0], "X");
        assert_eq!(cells[2], "AA");
        assert_eq!(cells[3], "");
    }

    #[test]
    fn or_unknown_handles_missing_and_empty() {
        assert_eq!(or_unknown(None), "Unknown");
        assert_eq!(or_unknown(Some("")), "Unknown");
        assert_eq!(or_unknown(Some("v1")), "v1");
    }
}
