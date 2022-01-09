use anyhow::Context as _;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct SimulatorDevices {
    devices: HashMap<String, serde_json::Value>,
}

impl SimulatorDevices {
    pub fn ios(&self) -> anyhow::Result<Vec<SimulatorDevice>> {
        let mut all_devices = vec![];

        let ios_keys = {
            let mut ios_keys = vec![];
            for key in self.devices.keys() {
                if key.contains(".iOS") {
                    ios_keys.push(key);
                }
            }

            ios_keys
        };

        for key in ios_keys {
            if let Some(raw_devices) = self.devices.get(key) {
                let devices: Vec<SimulatorDevice> = serde_json::from_value(raw_devices.clone())
                    .with_context(|| format!("Failed to parse raw_devices {:?}", raw_devices))?;
                all_devices.extend_from_slice(&devices);
            }
        }

        Ok(all_devices)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SimulatorDevice {
    pub udid: String,
    pub name: String,
    pub state: DeviceState,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum DeviceState {
    Shutdown,
    Booted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_json() {
        let contents = r#"{
            "devices": {
              "com.apple.CoreSimulator.SimRuntime.tvOS-15-2": [],
              "com.apple.CoreSimulator.SimRuntime.watchOS-8-3": [],
              "com.apple.CoreSimulator.SimRuntime.iOS-15-2": [
                {
                  "dataPath": "",
                  "dataPathSize": 859213824,
                  "logPath": "",
                  "udid": "4F57337E-1AF2-4D30-9726-87040063C016",
                  "isAvailable": true,
                  "logPathSize": 385024,
                  "deviceTypeIdentifier": "com.apple.CoreSimulator.SimDeviceType.iPhone-8",
                  "state": "Booted",
                  "name": "iPhone 8"
                },
                {
                    "dataPath" : "",
                    "dataPathSize" : 13312000,
                    "logPath" : "",
                    "udid" : "4F8AC01F-F4AD-4550-A853-C535C0BA7AF0",
                    "isAvailable" : true,
                    "deviceTypeIdentifier" : "com.apple.CoreSimulator.SimDeviceType.iPhone-8-Plus",
                    "state" : "Shutdown",
                    "name" : "iPhone 8 Plus"
                }
              ]
            }
          }"#;
        let devices: SimulatorDevices = serde_json::from_str(&contents).unwrap();
        let ios_devices = devices.ios().unwrap();

        assert_eq!(
            ios_devices,
            vec![
                SimulatorDevice {
                    udid: "4F57337E-1AF2-4D30-9726-87040063C016".into(),
                    name: "iPhone 8".to_string(),
                    state: DeviceState::Booted
                },
                SimulatorDevice {
                    udid: "4F8AC01F-F4AD-4550-A853-C535C0BA7AF0".into(),
                    name: "iPhone 8 Plus".to_string(),
                    state: DeviceState::Shutdown
                }
            ]
        )
    }
}
