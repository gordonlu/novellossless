pub struct MetricRegistry;

impl MetricRegistry {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}
