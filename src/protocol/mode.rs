//! Discovery modes: which vendor protocol a scan speaks.

use std::net::Ipv4Addr;

use super::device::DiscoveredDevice;
use super::{mndp, qnap, request, sadp, tlv};

/// Vendor protocol used by a scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Ubiquiti,
    MikroTik,
    Hikvision,
    Qnap,
}

/// Multicast group a mode must join to hear replies.
pub struct MulticastGroup {
    pub group: Ipv4Addr,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Ubiquiti, Mode::MikroTik, Mode::Hikvision, Mode::Qnap];

    /// Label shown in the mode selector.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Ubiquiti => "Ubiquiti",
            Mode::MikroTik => "MikroTik (MNDP)",
            Mode::Hikvision => "CCTV / NVR (SADP)",
            Mode::Qnap => "NAS (QNAP)",
        }
    }

    pub fn from_index(index: usize) -> Mode {
        Self::ALL.get(index).copied().unwrap_or_default()
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }

    /// UDP port probes are sent to (and replies arrive on).
    pub fn port(self) -> u16 {
        match self {
            Mode::Ubiquiti => request::PORT,
            Mode::MikroTik => mndp::PORT,
            Mode::Hikvision => sadp::PORT,
            Mode::Qnap => qnap::PORT,
        }
    }

    /// Probe datagrams for one send round (SADP embeds a fresh GUID each call).
    pub fn probes(self) -> Vec<Vec<u8>> {
        match self {
            Mode::Ubiquiti => request::ALL.iter().map(|p| p.to_vec()).collect(),
            Mode::MikroTik => vec![mndp::PROBE.to_vec()],
            Mode::Hikvision => vec![sadp::probe()],
            Mode::Qnap => qnap::PROBES.iter().map(|p| p.to_vec()).collect(),
        }
    }

    /// Group to join and to send the probe to, if the protocol is multicast.
    pub fn multicast(self) -> Option<MulticastGroup> {
        (self == Mode::Hikvision).then_some(MulticastGroup {
            group: sadp::GROUP,
        })
    }

    /// Default web UI port when the device did not report one.
    pub fn default_web_port(self) -> Option<u16> {
        match self {
            Mode::Qnap => Some(qnap::WEB_PORT),
            _ => None,
        }
    }

    /// Titles of the six list columns.
    pub fn columns(self) -> [&'static str; 6] {
        match self {
            Mode::Ubiquiti => [
                "Product Name",
                "IP Address",
                "Hardware Address",
                "System Name",
                "SSID",
                "Firmware Version",
            ],
            Mode::MikroTik => [
                "Board",
                "IP Address",
                "MAC Address",
                "Identity",
                "Platform",
                "Version",
            ],
            Mode::Hikvision => [
                "Device Type",
                "IP Address",
                "MAC Address",
                "Serial No.",
                "Status",
                "Software Version",
            ],
            Mode::Qnap => [
                "Model",
                "IP Address",
                "MAC Address",
                "Server Name",
                "Type",
                "Firmware Version",
            ],
        }
    }

    /// Parse one datagram into a device (sender IPv4 fills a missing address).
    pub fn parse(self, data: &[u8], sender: Ipv4Addr) -> Option<DiscoveredDevice> {
        let mut device = match self {
            Mode::Ubiquiti => tlv::parse(data),
            Mode::MikroTik => mndp::parse(data),
            Mode::Hikvision => sadp::parse(data),
            Mode::Qnap => qnap::parse(data),
        }?;
        if device.ip.is_none() {
            device.ip = Some(sender);
        }
        if device.web_port.is_none() {
            device.web_port = self.default_web_port();
        }
        Some(device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trips_and_clamps() {
        for m in Mode::ALL {
            assert_eq!(Mode::from_index(m.index()), m);
        }
        assert_eq!(Mode::from_index(99), Mode::Ubiquiti);
    }

    #[test]
    fn every_mode_has_probes_and_six_columns() {
        for m in Mode::ALL {
            assert!(!m.probes().is_empty());
            assert!(m.probes().iter().all(|p| !p.is_empty()));
            assert_eq!(m.columns().len(), 6);
        }
    }

    #[test]
    fn only_sadp_is_multicast() {
        assert!(Mode::Hikvision.multicast().is_some());
        assert!(Mode::MikroTik.multicast().is_none());
    }
}
