pub trait McpClient: Send + Sync {
    fn name(&self) -> &'static str;
}

pub trait McpServer: Send + Sync {
    fn name(&self) -> &'static str;
}
