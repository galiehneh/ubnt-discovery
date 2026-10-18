//! JNI bridge: `com.galiehneh.ubntdiscovery.NativeBridge`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use jni::errors::Error;
use jni::objects::JObject;
use jni::{jni_sig, jni_str, EnvUnowned, JValue};
use serde_json::{json, Value};
use ubnt_discovery::protocol::client;
use ubnt_discovery::protocol::device::DiscoveredDevice;

static CANCEL: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

/// Serialize one device to the JSON shape the Kotlin side expects.
pub fn device_to_json(d: &DiscoveredDevice) -> String {
    let addresses: Vec<Value> = d
        .addresses
        .iter()
        .map(|a| json!({ "mac": a.mac, "ip": a.ip.map(|ip| ip.to_string()) }))
        .collect();
    json!({
        "mac": d.mac,
        "ip": d.ip.map(|ip| ip.to_string()),
        "hostname": d.hostname,
        "essid": d.essid,
        "firmware": d.firmware,
        "model_code": d.model_code,
        "product": d.display_product(),
        "uptime": d.display_uptime(),
        "addresses": addresses,
    })
    .to_string()
}

/// Blocking scan; `listener.onDevice(json)` is invoked from scan worker threads.
#[no_mangle]
pub extern "system" fn Java_com_galiehneh_ubntdiscovery_NativeBridge_scan<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    duration_ms: i64,
    listener: JObject<'caller>,
) {
    let outcome = unowned_env.with_env(|env| -> Result<(), Error> {
        let vm = env.get_java_vm()?;
        let listener = env.new_global_ref(listener)?;
        let ms = u64::try_from(duration_ms).unwrap_or(0);
        // A cancel() that landed before this point stays set so the scan returns at once;
        // the flag is cleared only after the scan ends.
        client::scan(Duration::from_millis(ms), CANCEL.clone(), move |dev| {
            let json = device_to_json(&dev);
            // Errors (e.g. listener threw) are dropped; scanning continues.
            let _ = vm.attach_current_thread(|env| -> Result<(), Error> {
                let jstr = env.new_string(json)?;
                env.call_method(
                    &listener,
                    jni_str!("onDevice"),
                    jni_sig!((json: JString) -> void),
                    &[JValue::Object(&jstr)],
                )?;
                Ok(())
            });
        });
        CANCEL.store(false, Ordering::SeqCst);
        Ok(())
    });
    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

/// Request the running scan to stop.
#[no_mangle]
pub extern "system" fn Java_com_galiehneh_ubntdiscovery_NativeBridge_cancel<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    CANCEL.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::SystemTime;
    use ubnt_discovery::protocol::device::DeviceAddress;

    #[test]
    fn json_has_expected_fields() {
        let d = DiscoveredDevice {
            mac: "AA:BB".into(),
            ip: Some(Ipv4Addr::new(10, 0, 0, 2)),
            hostname: Some("h".into()),
            essid: None,
            firmware: Some("fw".into()),
            model_code: Some("N5N".into()),
            product_name: None,
            uptime_seconds: Some(86_400),
            addresses: vec![DeviceAddress {
                mac: "AA:BB".into(),
                ip: None,
            }],
            last_seen: SystemTime::UNIX_EPOCH,
        };
        let v: Value = serde_json::from_str(&device_to_json(&d)).unwrap();
        assert_eq!(v["mac"], "AA:BB");
        assert_eq!(v["ip"], "10.0.0.2");
        assert_eq!(v["hostname"], "h");
        assert!(v["essid"].is_null());
        assert_eq!(v["product"], "N5N");
        assert_eq!(v["uptime"], "1 day 00:00:00");
        assert_eq!(v["addresses"][0]["mac"], "AA:BB");
        assert!(v["addresses"][0]["ip"].is_null());
    }
}
