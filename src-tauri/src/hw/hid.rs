//! The keyboard's LampArray HID interface (VID 0414, PID 8105, interface 4).

use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Devices::HumanInterfaceDevice::{HidD_GetHidGuid, HidD_SetFeature};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

use crate::keyboard::LampDevice;

// Interface 4 of the keyboard's composite USB device carries usage page 0x59.
const LAMP_INTERFACE: &str = "vid_0414&pid_8105&mi_04";

pub struct HidLamp {
    handle: HANDLE,
}

// The handle is only used from the lamp thread that owns it.
unsafe impl Send for HidLamp {}

impl LampDevice for HidLamp {
    fn write(&mut self, report: &[u8]) -> Result<(), String> {
        unsafe { HidD_SetFeature(self.handle, report.as_ptr() as *const _, report.len() as u32) }
            .then_some(())
            .ok_or_else(|| windows::core::Error::from_win32().message())
    }
}

impl Drop for HidLamp {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

fn lamp_path() -> Option<Vec<u16>> {
    unsafe {
        let guid = HidD_GetHidGuid();
        let set = SetupDiGetClassDevsW(Some(&guid), PCWSTR::null(), None, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE).ok()?;
        let mut found = None;
        let mut index = 0;
        loop {
            let mut iface = SP_DEVICE_INTERFACE_DATA { cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInterfaces(set, None, &guid, index, &mut iface).is_err() {
                break;
            }
            index += 1;
            let mut needed = 0u32;
            let _ = SetupDiGetDeviceInterfaceDetailW(set, &iface, None, 0, Some(&mut needed), None);
            if needed == 0 {
                continue;
            }
            // u32 storage keeps the detail struct aligned.
            let mut buf = vec![0u32; (needed as usize).div_ceil(4)];
            let detail = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(set, &iface, Some(detail), needed, None, None).is_err() {
                continue;
            }
            let path = PCWSTR((*detail).DevicePath.as_ptr());
            let text = path.to_string().unwrap_or_default();
            if text.to_lowercase().contains(LAMP_INTERFACE) {
                let mut wide: Vec<u16> = text.encode_utf16().collect();
                wide.push(0);
                found = Some(wide);
                break;
            }
        }
        let _ = SetupDiDestroyDeviceInfoList(set);
        found
    }
}

pub fn open_lamp() -> Option<Box<dyn LampDevice>> {
    let path = lamp_path()?;
    let handle = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
    .ok()?;
    Some(Box::new(HidLamp { handle }))
}
