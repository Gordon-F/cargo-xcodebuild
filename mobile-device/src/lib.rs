use anyhow::Context;
use core_foundation::{
    array::{CFArrayGetCount, CFArrayGetValues},
    base::{CFRange, TCFType},
    dictionary::CFDictionary,
    string::CFString,
};
use std::mem;

mod ffi;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DeviceConnectionType {
    Usb,
    Network,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MobileDevice {
    ptr: ffi::AMDeviceRef,
    pub identifier: String,
    pub device_name: String,
    pub connection_type: DeviceConnectionType,
    pub cpu_architecture: String,
    pub device_class: String,
    pub product_version: String,
    pub model: String,
}

struct Session(ffi::AMDeviceRef);

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                let _ = ffi::AMDeviceStopSession(self.0);
                let _ = ffi::AMDeviceDisconnect(self.0);
            }
        }
    }
}

impl MobileDevice {
    pub fn install_app(&self, app_path: &std::path::Path) -> anyhow::Result<()> {
        log::trace!(
            "Installing app: {:?} to device {}",
            app_path,
            self.identifier
        );
        if !app_path.exists() || !app_path.is_dir() {
            anyhow::bail!("AppPath is not a dir or not exists: {:?}", app_path)
        }
        let path_str = app_path
            .to_str()
            .with_context(|| "Install_app. Failed to convert path to str".to_string())?;
        let url =
            ::core_foundation::url::CFURL::from_file_system_path(CFString::new(path_str), 0, true);
        let options = [(
            CFString::from_static_string("PackageType"),
            CFString::from_static_string("Developper").as_CFType(),
        )];
        let options = CFDictionary::from_CFType_pairs(&options);

        unsafe {
            let session = self.start_session()?;
            log::trace!("AMDeviceSecureTransferPath...");
            check_native_return_code(ffi::AMDeviceSecureTransferPath(
                0,
                self.ptr,
                url.as_concrete_TypeRef(),
                options.as_concrete_TypeRef(),
                std::ptr::null(),
                std::ptr::null(),
            ))
            .with_context(|| "Install_app.AMDeviceSecureTransferPath".to_string())?;
            drop(session);

            let session = self.start_session()?;
            log::trace!("AMDeviceSecureInstallApplication...");
            check_native_return_code(ffi::AMDeviceSecureInstallApplication(
                0,
                self.ptr,
                url.as_concrete_TypeRef(),
                options.as_concrete_TypeRef(),
                std::ptr::null(),
                std::ptr::null(),
            ))
            .with_context(|| "InstallApp.AMDeviceSecureInstallApplication".to_string())?;
            drop(session);
        }

        Ok(())
    }

    fn from_raw_ptr(ptr: ffi::AMDeviceRef) -> anyhow::Result<Self> {
        unsafe {
            check_native_return_code(ffi::AMDeviceConnect(ptr))
                .with_context(|| "MobileDevice.AMDeviceConnect".to_string())?;

            let raw_device_id: CFString =
                TCFType::wrap_under_get_rule(ffi::AMDeviceCopyDeviceIdentifier(ptr));
            let raw_interface_type = ffi::AMDeviceGetInterfaceType(ptr);

            // 0=Unknown, 1 = Direct/USB, 2 = Indirect/WIFI, 3 = Companion proxy
            let connection_type = match raw_interface_type {
                1 => DeviceConnectionType::Usb,
                2 => DeviceConnectionType::Network,
                _ => {
                    anyhow::bail!("Unknown device interface type: {}", raw_interface_type);
                }
            };

            let device_name = Self::copy_device_value(ptr, "DeviceName").unwrap();
            let cpu_architecture = Self::copy_device_value(ptr, "CPUArchitecture").unwrap();
            let device_class = Self::copy_device_value(ptr, "DeviceClass").unwrap();
            let product_version = Self::copy_device_value(ptr, "ProductVersion").unwrap();
            let model = Self::copy_device_value(ptr, "HardwareModel").unwrap();

            check_native_return_code(ffi::AMDeviceDisconnect(ptr))
                .with_context(|| "MobileDevice.AMDeviceDisconnect".to_string())?;

            Ok(Self {
                ptr,
                identifier: format!("{}", raw_device_id),
                device_name,
                connection_type,
                cpu_architecture,
                device_class,
                product_version,
                model,
            })
        }
    }

    fn start_session(&self) -> anyhow::Result<Session> {
        unsafe {
            log::trace!("Starting session with device {}", self.identifier);
            check_native_return_code(ffi::AMDeviceConnect(self.ptr))
                .with_context(|| "start_session.AMDeviceConnect")?;
            if ffi::AMDeviceIsPaired(self.ptr) == 0 {
                anyhow::bail!("Device is not paired. Are you trust this computer? :)");
            }
            check_native_return_code(ffi::AMDeviceValidatePairing(self.ptr))
                .with_context(|| "start_session.AMDeviceValidatePairing")?;
            check_native_return_code(ffi::AMDeviceStartSession(self.ptr))
                .with_context(|| "start_session.AMDeviceStartSession")?;

            Ok(Session(self.ptr))
        }
    }

    fn copy_device_value(ptr: ffi::AMDeviceRef, property: &'static str) -> Option<String> {
        unsafe {
            let key = CFString::from_static_string(property);
            let raw_value =
                ffi::AMDeviceCopyValue(ptr, std::ptr::null(), key.as_concrete_TypeRef());
            if raw_value.is_null() {
                log::error!("Value from property `{}` is null", property);
                return None;
            }
            let value: CFString = TCFType::wrap_under_get_rule(mem::transmute(raw_value));

            Some(format!("{}", value))
        }
    }
}

/// Return a list of connected devices
pub fn get_device_list() -> Vec<MobileDevice> {
    unsafe {
        let devices_ptr = ffi::AMDCreateDeviceList();
        let count = CFArrayGetCount(devices_ptr);
        let mut raw_devices = Vec::with_capacity(count as usize);
        CFArrayGetValues(
            devices_ptr,
            CFRange {
                location: 0,
                length: count,
            },
            raw_devices.as_mut_ptr(),
        );
        raw_devices.set_len(count as usize);

        let mut typed_devices: Vec<MobileDevice> = Vec::with_capacity(count as usize);

        for pointer in raw_devices {
            if let Ok(device) = MobileDevice::from_raw_ptr(pointer as ffi::AMDeviceRef) {
                typed_devices.push(device);
            }
        }

        typed_devices
    }
}

fn check_native_return_code(code: std::os::raw::c_int) -> anyhow::Result<()> {
    match code as u32 {
        0x00000000 => {}
        0xe8000008 => {
            anyhow::bail!("The file could not be found. kAMDNotFoundError")
        }
        0xe8008015 => {
            anyhow::bail!("A valid provisioning profile for this executable was not found.")
        }
        0xe8000065 => {
            anyhow::bail!("Could not connect to the device. kAMDMuxConnectError")
        }
        0xe800000b => {
            anyhow::bail!("Not connected to the device. kAMDNotConnectedError")
        }
        0xe8008021 => {
            anyhow::bail!(
                "The maximum number of apps for free development profiles has been reached."
            )
        }
        // TODO:
        _ => {
            log::error!("Unknown error code: 0x{:x}", code);
            anyhow::bail!("Unknown error code: 0x{:x}", code)
        }
    };

    Ok(())
}
