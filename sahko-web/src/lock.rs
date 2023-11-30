use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WriteLock(Arc<Mutex<()>>);

impl WriteLock {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(())))
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.0.lock().unwrap()
    }
}
