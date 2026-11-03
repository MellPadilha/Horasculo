use std::{sync::Arc, time::Instant};

#[derive(Clone)]
pub struct AppState {
    pub started_at: Arc<Instant>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            started_at: Arc::new(Instant::now()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
