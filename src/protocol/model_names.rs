use std::collections::HashMap;
use std::sync::OnceLock;

/// (code, name) pairs in original order (Ubiquiti Discovery Tool v2.5.1).
const TABLE: &[(&str, &str)] = &[
    ("AB5-GPS", "Airbeam 5 GPS"),
    ("P5X-GPS", "PowerBridge M5 X3 GPS"),
    ("R5X-GPS", "Rocket M5 X3 GPS"),
    ("R36-GPS", "Rocket M365 GPS"),
    ("RM3-GPS", "Rocket M3 GPS"),
    ("R2N-GPS", "Rocket M2 GPS"),
    ("R5N-GPS", "Rocket M5 GPS"),
    ("R5T-GPS", "Rocket M5 Titanium GPS"),
    ("R9N-GPS", "Rocket M900 GPS"),
    ("AG2-HP", "AirGrid M2 HP"),
    ("BS5-HP", "Bullet5 HP"),
    ("pS2-HP", "PicoStation2 HP"),
    ("LS2", "LiteStation2"),
    ("PS2", "PowerStation2"),
    ("NS2", "NanoStation2"),
    ("MS2", "MiniStation2"),
    ("TS2", "TravelStation2"),
    ("BS2", "Bullet2"),
    ("BH2", "Bullet2 Hi-Power"),
    ("BM2", "BulletStation2 Mini"),
    ("LC2", "NanoStation2 L"),
    ("LS9", "LiteStation9"),
    ("pS2", "PicoStation2"),
    ("AP1", "AP-1000"),
    ("LS5", "LiteStation5"),
    ("PS5", "PowerStation5"),
    ("NS5", "NanoStation5"),
    ("MS5", "WispStation5"),
    ("BS5", "Bullet5"),
    ("LC5", "NanoStation5 L"),
    ("pS5", "PicoStation5"),
    ("NS3", "NanoStation3"),
    ("AGW", "airGateway"),
    ("M25", "LiteStation M25"),
    ("BZ2", "UniFi AP"),
    ("UGW", "UniTel Gateway"),
    ("AF24", "airFiber 24G"),
    ("M2M", "mFi mPort"),
    ("M2S", "mFi mPort Serial"),
    ("P1U", "mFi mPower Mini"),
    ("P3U", "mFi mPower"),
    ("P8U", "mFi mPower Pro"),
    ("P1E", "mFi mPower Mini"),
    ("P3E", "mFi mPower"),
    ("P6E", "mFi mPower Pro"),
    ("IWO2U", "mFi mOutlet"),
    ("IWS1U", "mFi mSwitch"),
    ("IWD1U", "mFi mDimmer"),
    ("RTR", "EdgeRouter"),
    ("ERLite-3", "EdgeRouter Lite"),
    ("N2N", "NanoStation M2"),
    ("N5N", "NanoStation M5"),
    ("N6N", "NanoStation M6"),
    ("R2N", "Rocket M2"),
    ("R2T", "Rocket M2 Titanium"),
    ("R5N", "Rocket M5"),
    ("B2N", "Bullet M2"),
    ("B2T", "Bullet M2 Titanium"),
    ("B5N", "Bullet M5"),
    ("B5T", "Bullet M5 Titanium"),
    ("AG2", "AirGrid M2"),
    ("AG5-HP", "AirGrid M5 HP"),
    ("AG5", "AirGrid M5"),
    ("p2N", "PicoStation M2"),
    ("p5N", "PicoStation M5"),
    ("AW5", "AirWire"),
    ("LM2", "NanoStation Loco M2"),
    ("LM5", "NanoStation Loco M5"),
    ("M25", "LiteStation M25"),
    ("PAP", "PowerAP N"),
    ("LAP-HP", "AirRouter HP"),
    ("LAP", "AirRouter"),
    ("AMG", "AirMax Gateway"),
    ("PB5", "PowerBridge M5"),
    ("NB5", "NanoBridge M5"),
    ("NB2", "NanoBridge M2"),
    ("RM3", "Rocket M3"),
    ("NS3", "NanoStation M3"),
    ("PB3", "PowerBridge M3"),
    ("NB3", "NanoBridge M3"),
    ("R36", "Rocket M365"),
    ("N36", "NanoStation M365"),
    ("P36", "PowerBridge M365"),
    ("B36", "NanoBridge M365"),
    ("R9N", "Rocket M900"),
    ("N9N", "NanoStation Loco M900"),
    ("N9S", "NanoStation M900"),
    ("NB9", "NanoBridge M900"),
    ("PBM10", "PowerBridge M10"),
    ("PB5", "PowerBridge M5"),
    ("SM5", "LiteStation M5"),
    ("WM5", "WispStation M5"),
    ("3GS", "3G Station"),
    ("3GP", "3G Station Professional"),
    ("3GO", "3G Station Outdoor"),
    ("AirCamDomeIR", "airCam Dome IR"),
    ("AirCamBulletIR", "airCam Bullet IR"),
    ("AirCamPro", "airCam PRO"),
    ("ADI", "airCam Dome IR"),
    ("ABI", "airCam Bullet IR"),
    ("ACP", "airCam PRO"),
    ("AirCamPRO", "airCam PRO"),
    ("AirCamDome", "airCam Dome"),
    ("AirCamMini", "airCam Mini"),
    ("AirCam", "airCam"),
    ("NVR", "airVision NVR"),
    ("ACD", "airCam Dome"),
    ("ACM", "airCam Mini"),
    ("AC", "airCam"),
    ("TSW-5-POE", "TOUGHSwitch 5 PoE"),
    ("TSW-8-POE", "TOUGHSwitch 8 PoE"),
    ("TSW-8", "TOUGHSwitch 8"),
    ("TSW-PoE PRO", "TOUGHSwitch PoE PRO"),
    ("TSW-PoE", "TOUGHSwitch PoE"),
];

fn lookup() -> &'static HashMap<String, &'static str> {
    static MAP: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = HashMap::new();
        for (code, name) in TABLE {
            // First entry wins, matching the original linear scan.
            map.entry(code.to_lowercase()).or_insert(*name);
        }
        map
    })
}

/// Resolve a platform code to a product name. Returns the trimmed code itself
/// when unmapped, `None` when blank. Table: 114 entries, first entry wins.
pub fn resolve(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return None;
    }
    let name = lookup().get(&trimmed.to_lowercase()).copied();
    Some(name.unwrap_or(trimmed).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_114_entries() {
        assert_eq!(TABLE.len(), 114);
    }

    #[test]
    fn resolves_known_code() {
        assert_eq!(resolve("N5N").as_deref(), Some("NanoStation M5"));
    }

    #[test]
    fn maps_known_codes() {
        for (code, expected) in [
            ("N5N", "NanoStation M5"),
            ("BZ2", "UniFi AP"),
            ("ERLite-3", "EdgeRouter Lite"),
            ("R5N", "Rocket M5"),
            ("n5n", "NanoStation M5"),
        ] {
            assert_eq!(resolve(code).as_deref(), Some(expected), "{code}");
        }
    }

    #[test]
    fn returns_code_itself_when_unknown() {
        assert_eq!(resolve("ZZZ").as_deref(), Some("ZZZ"));
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(resolve("n5n").as_deref(), Some("NanoStation M5"));
        assert_eq!(resolve("PS2").as_deref(), Some("PowerStation2"));
        assert_eq!(resolve("PS2-HP").as_deref(), Some("PicoStation2 HP"));
    }

    #[test]
    fn first_entry_wins_on_duplicates() {
        assert_eq!(resolve("NS3").as_deref(), Some("NanoStation3"));
        assert_eq!(resolve("M25").as_deref(), Some("LiteStation M25"));
        assert_eq!(resolve("PB5").as_deref(), Some("PowerBridge M5"));
    }

    #[test]
    fn unknown_code_returns_trimmed_code() {
        assert_eq!(resolve("  ZZZ9 ").as_deref(), Some("ZZZ9"));
        assert_eq!(resolve("ZZZ9").as_deref(), Some("ZZZ9"));
    }

    #[test]
    fn blank_code_returns_none() {
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("   \t"), None);
    }

    #[test]
    fn trims_before_lookup() {
        assert_eq!(resolve(" N5N ").as_deref(), Some("NanoStation M5"));
    }
}
