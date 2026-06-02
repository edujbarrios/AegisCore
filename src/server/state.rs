use crate::config::AegisConfig;
use crate::registry::{SkillRegistry, ToolContext, ToolRegistry};
use crate::skills::SkillStore;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub cfg: AegisConfig,
    pub tools: Arc<ToolRegistry>,
    pub skills: Arc<RwLock<SkillRegistry>>,
    pub tool_ctx: ToolContext,
    pub store: SkillStore,
    pub rate_limiter: Arc<Mutex<RateLimiter>>,
}

#[derive(Debug)]
pub struct RateLimiter {
    window: Duration,
    max_events: usize,
    events: std::collections::VecDeque<Instant>,
}

impl RateLimiter {
    pub fn new(window: Duration, max_events: usize) -> Self {
        Self {
            window,
            max_events,
            events: std::collections::VecDeque::new(),
        }
    }

    pub fn allow(&mut self) -> bool {
        let now = Instant::now();
        while let Some(front) = self.events.front().copied() {
            if now.duration_since(front) <= self.window {
                break;
            }
            self.events.pop_front();
        }
        if self.events.len() >= self.max_events {
            return false;
        }
        self.events.push_back(now);
        true
    }
}
