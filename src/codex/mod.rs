pub trait CodexAdapter: Send + Sync {
    fn name(&self) -> &'static str;
}
