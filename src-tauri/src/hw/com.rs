//! Real `GbWmi` backed by COM WMI (`root\WMI`). COM interfaces are not
//! `Send`, so one dedicated thread owns them and every call is marshalled to
//! it over a channel. That also serialises all firmware calls.

use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::thread;

use windows::core::{BSTR, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator, CIM_UINT8, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_FLAG_RETURN_WBEM_COMPLETE, WBEM_INFINITE,
};

use super::{GbWmi, HwError};

const SET_CLASS: &str = "GB_WMIACPI_Set";
const GET_CLASS: &str = "GB_WMIACPI_Get";

enum Request {
    Set(String, u32, Sender<Result<(), HwError>>),
    Get(String, Sender<Result<u32, HwError>>),
}

pub struct ComWmi {
    tx: Mutex<Sender<Request>>,
}

impl ComWmi {
    pub fn connect() -> Result<ComWmi, HwError> {
        let (tx, rx) = channel::<Request>();
        let (ready_tx, ready_rx) = channel::<Result<(), HwError>>();
        thread::Builder::new()
            .name("wmi".into())
            .spawn(move || {
                let session = match Session::open() {
                    Ok(s) => {
                        let _ = ready_tx.send(Ok(()));
                        s
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                for req in rx {
                    match req {
                        Request::Set(m, v, reply) => {
                            let _ = reply.send(session.set(&m, v));
                        }
                        Request::Get(m, reply) => {
                            let _ = reply.send(session.get(&m));
                        }
                    }
                }
            })
            .map_err(|e| HwError::Unavailable(e.to_string()))?;
        ready_rx
            .recv()
            .map_err(|_| HwError::Unavailable("wmi thread exited".into()))??;
        Ok(ComWmi { tx: Mutex::new(tx) })
    }

    fn send(&self, req: Request) -> Result<(), HwError> {
        self.tx
            .lock()
            .unwrap()
            .send(req)
            .map_err(|_| HwError::Unavailable("wmi thread exited".into()))
    }
}

impl GbWmi for ComWmi {
    fn set(&self, method: &str, value: u32) -> Result<(), HwError> {
        let (reply, rx) = channel();
        self.send(Request::Set(method.to_string(), value, reply))?;
        rx.recv().map_err(|_| HwError::Unavailable("wmi thread exited".into()))?
    }

    fn get(&self, method: &str) -> Result<u32, HwError> {
        let (reply, rx) = channel();
        self.send(Request::Get(method.to_string(), reply))?;
        rx.recv().map_err(|_| HwError::Unavailable("wmi thread exited".into()))?
    }
}

/// Lives on the WMI thread only.
struct Session {
    svc: IWbemServices,
    set_class: IWbemClassObject,
    get_class: IWbemClassObject,
    set_path: BSTR,
    get_path: BSTR,
}

fn unavailable(e: windows::core::Error) -> HwError {
    HwError::Unavailable(e.message())
}

fn call_err(method: &str, detail: impl Into<String>) -> HwError {
    HwError::Call { method: method.to_string(), detail: detail.into() }
}

impl Session {
    fn open() -> Result<Session, HwError> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(unavailable)?;
            let locator: IWbemLocator =
                CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).map_err(unavailable)?;
            let svc = locator
                .ConnectServer(
                    &BSTR::from("root\\WMI"),
                    &BSTR::new(),
                    &BSTR::new(),
                    &BSTR::new(),
                    0,
                    &BSTR::new(),
                    None,
                )
                .map_err(unavailable)?;
            CoSetProxyBlanket(
                &svc,
                RPC_C_AUTHN_WINNT,
                RPC_C_AUTHZ_NONE,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )
            .map_err(unavailable)?;
            let set_class = class_object(&svc, SET_CLASS)?;
            let get_class = class_object(&svc, GET_CLASS)?;
            let set_path = first_instance_path(&svc, SET_CLASS)?;
            let get_path = first_instance_path(&svc, GET_CLASS)?;
            Ok(Session { svc, set_class, get_class, set_path, get_path })
        }
    }

    fn set(&self, method: &str, value: u32) -> Result<(), HwError> {
        unsafe {
            let mut in_sig: Option<IWbemClassObject> = None;
            self.set_class
                .GetMethod(&HSTRING::from(method), 0, &mut in_sig, std::ptr::null_mut())
                .map_err(|e| call_err(method, e.message()))?;
            let in_sig = in_sig.ok_or_else(|| call_err(method, "method has no input"))?;
            let input = in_sig.SpawnInstance(0).map_err(|e| call_err(method, e.message()))?;

            let mut cim_type = 0i32;
            let mut probe = VARIANT::default();
            input
                .Get(&HSTRING::from("Data"), 0, &mut probe, Some(&mut cim_type), None)
                .map_err(|e| call_err(method, e.message()))?;
            // WMI wants VT_UI1 for uint8 and VT_I4 for the wider unsigned types.
            let arg = if cim_type == CIM_UINT8.0 {
                VARIANT::from(u8::try_from(value).map_err(|_| call_err(method, "value exceeds uint8"))?)
            } else {
                VARIANT::from(value as i32)
            };
            input
                .Put(&HSTRING::from("Data"), 0, &arg, 0)
                .map_err(|e| call_err(method, e.message()))?;

            let mut out: Option<IWbemClassObject> = None;
            self.svc
                .ExecMethod(
                    &self.set_path,
                    &BSTR::from(method),
                    WBEM_FLAG_RETURN_WBEM_COMPLETE,
                    None,
                    &input,
                    Some(&mut out),
                    None,
                )
                .map_err(|e| call_err(method, e.message()))?;
            check_return(method, out.as_ref())
        }
    }

    fn get(&self, method: &str) -> Result<u32, HwError> {
        unsafe {
            let mut in_sig: Option<IWbemClassObject> = None;
            self.get_class
                .GetMethod(&HSTRING::from(method), 0, &mut in_sig, std::ptr::null_mut())
                .map_err(|e| call_err(method, e.message()))?;
            let input = match in_sig {
                Some(sig) => Some(sig.SpawnInstance(0).map_err(|e| call_err(method, e.message()))?),
                None => None,
            };
            let mut out: Option<IWbemClassObject> = None;
            self.svc
                .ExecMethod(
                    &self.get_path,
                    &BSTR::from(method),
                    WBEM_FLAG_RETURN_WBEM_COMPLETE,
                    None,
                    input.as_ref(),
                    Some(&mut out),
                    None,
                )
                .map_err(|e| call_err(method, e.message()))?;
            check_return(method, out.as_ref())?;
            let out = out.ok_or_else(|| call_err(method, "no output"))?;
            let mut data = VARIANT::default();
            out.Get(&HSTRING::from("Data"), 0, &mut data, None, None)
                .map_err(|e| call_err(method, e.message()))?;
            let v = i32::try_from(&data).map_err(|e| call_err(method, e.message()))?;
            u32::try_from(v).map_err(|_| call_err(method, format!("negative value {v}")))
        }
    }
}

/// These firmware methods return a boolean `ReturnValue`; false means the
/// BIOS rejected the call even though WMI itself succeeded.
unsafe fn check_return(method: &str, out: Option<&IWbemClassObject>) -> Result<(), HwError> {
    let Some(out) = out else { return Ok(()) };
    let mut rv = VARIANT::default();
    if out.Get(&HSTRING::from("ReturnValue"), 0, &mut rv, None, None).is_err() {
        return Ok(());
    }
    match bool::try_from(&rv) {
        Ok(false) => Err(call_err(method, "firmware returned false")),
        _ => Ok(()),
    }
}

unsafe fn class_object(svc: &IWbemServices, class: &str) -> Result<IWbemClassObject, HwError> {
    let mut obj: Option<IWbemClassObject> = None;
    svc.GetObject(&BSTR::from(class), WBEM_FLAG_RETURN_WBEM_COMPLETE, None, Some(&mut obj), None)
        .map_err(|e| HwError::Unavailable(format!("{class}: {}", e.message())))?;
    obj.ok_or_else(|| HwError::Unavailable(format!("{class}: class missing")))
}

unsafe fn first_instance_path(svc: &IWbemServices, class: &str) -> Result<BSTR, HwError> {
    let instances = svc
        .CreateInstanceEnum(
            &BSTR::from(class),
            WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
            None,
        )
        .map_err(|e| HwError::Unavailable(format!("{class}: {}", e.message())))?;
    let mut objs = [None];
    let mut returned = 0u32;
    // Without elevation this is where WMI says "access denied"; surface it.
    instances
        .Next(WBEM_INFINITE, &mut objs, &mut returned)
        .ok()
        .map_err(|e| HwError::Unavailable(format!("{class}: {}", e.message())))?;
    let obj = objs[0]
        .take()
        .filter(|_| returned == 1)
        .ok_or_else(|| HwError::Unavailable(format!("{class}: no instance")))?;
    let mut path = VARIANT::default();
    obj.Get(&HSTRING::from("__RELPATH"), 0, &mut path, None, None)
        .map_err(|e| HwError::Unavailable(format!("{class}: {}", e.message())))?;
    BSTR::try_from(&path).map_err(|e| HwError::Unavailable(format!("{class}: {}", e.message())))
}
