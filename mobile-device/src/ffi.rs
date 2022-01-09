use std::os::raw;

use core_foundation::{
    array::CFArrayRef, dictionary::CFDictionaryRef, string::CFStringRef, url::CFURLRef,
};

pub type AMDeviceRef = *const raw::c_void;

extern "C" {
    pub fn AMDCreateDeviceList() -> CFArrayRef;
    pub fn AMDeviceCopyDeviceIdentifier(device: AMDeviceRef) -> CFStringRef;
    pub fn AMDeviceCopyValue(
        device: AMDeviceRef,
        domain: CFStringRef,
        key: CFStringRef,
    ) -> *const raw::c_void;
    pub fn AMDeviceGetInterfaceType(device: AMDeviceRef) -> raw::c_int;
    pub fn AMDeviceConnect(device: AMDeviceRef) -> raw::c_int;
    pub fn AMDeviceDisconnect(device: AMDeviceRef) -> raw::c_int;
    pub fn AMDeviceIsPaired(device: AMDeviceRef) -> raw::c_int;
    pub fn AMDeviceValidatePairing(device: AMDeviceRef) -> raw::c_int;
    pub fn AMDeviceStartSession(device: AMDeviceRef) -> raw::c_int;
    pub fn AMDeviceStopSession(device: AMDeviceRef) -> raw::c_int;
    pub fn AMDeviceSecureInstallApplication(
        zero: raw::c_int,
        device: AMDeviceRef,
        url: CFURLRef,
        options: CFDictionaryRef,
        callback: *const raw::c_void,
        cbarg: *const raw::c_void,
    ) -> raw::c_int;
    pub fn AMDeviceSecureTransferPath(
        zero: raw::c_int,
        device: AMDeviceRef,
        url: CFURLRef,
        options: CFDictionaryRef,
        callback: *const raw::c_void,
        cbarg: *const raw::c_void,
    ) -> raw::c_int;
}
