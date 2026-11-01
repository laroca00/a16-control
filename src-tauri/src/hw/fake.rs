use std::collections::HashMap;
use std::sync::Mutex;

use super::{GbWmi, HwError};

/// Records every successful setter call and serves getter values from a map.
#[derive(Default)]
pub struct FakeWmi {
    calls: Mutex<Vec<(String, u32)>>,
    values: Mutex<HashMap<String, u32>>,
    fail_on: Option<String>,
}

impl FakeWmi {
    pub fn failing_on(method: &str) -> Self {
        Self { fail_on: Some(method.to_string()), ..Self::default() }
    }

    pub fn calls(&self) -> Vec<(String, u32)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn put(&self, method: &str, value: u32) {
        self.values.lock().unwrap().insert(method.to_string(), value);
    }

    fn check(&self, method: &str) -> Result<(), HwError> {
        match &self.fail_on {
            Some(m) if m == method => Err(HwError::Call {
                method: method.to_string(),
                detail: "injected failure".into(),
            }),
            _ => Ok(()),
        }
    }
}

impl GbWmi for FakeWmi {
    fn set(&self, method: &str, value: u32) -> Result<(), HwError> {
        self.check(method)?;
        self.calls.lock().unwrap().push((method.to_string(), value));
        Ok(())
    }

    fn get(&self, method: &str) -> Result<u32, HwError> {
        self.check(method)?;
        Ok(*self.values.lock().unwrap().get(method).unwrap_or(&0))
    }
}
