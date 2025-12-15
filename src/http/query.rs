// ============================================================================
// Query Parameter Parser
// ============================================================================
//
// Simple zero-dependency parser to eliminate duplicate query string parsing
// logic across endpoints. Keeps binary size minimal while reducing duplication.

pub struct QueryParams<'a> {
    query: Option<&'a str>,
}

impl<'a> QueryParams<'a> {
    pub fn new(query: Option<&'a str>) -> Self {
        Self { query }
    }

    pub fn get(&self, key: &str) -> Option<&'a str> {
        self.query.and_then(|q| {
            for param in q.split('&') {
                if let Some((k, v)) = param.split_once('=') {
                    if k == key {
                        return Some(v);
                    }
                }
            }
            None
        })
    }

    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    pub fn get_usize(&self, key: &str, default: usize) -> usize {
        self.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    pub fn get_optional_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    pub fn get_optional_usize(&self, key: &str) -> Option<usize> {
        self.get(key).and_then(|v| v.parse().ok())
    }
}

/// Typed extractor for device state routes: `/api/devices/{id}/state`.
pub struct DeviceStatePath<'a> {
    pub device_id: &'a str,
}

impl<'a> DeviceStatePath<'a> {
    const PREFIX: &'static str = "/api/devices/";
    const SUFFIX: &'static str = "/state";

    pub fn parse(path: &'a str) -> Option<Self> {
        if path.starts_with(Self::PREFIX) && path.ends_with(Self::SUFFIX) {
            let id = &path[Self::PREFIX.len()..path.len() - Self::SUFFIX.len()];
            if !id.is_empty() {
                return Some(Self { device_id: id });
            }
        }
        None
    }
}

/// Helper for parsing `/api/automation/logs` query parameters.
pub struct ExecutionLogsQuery<'a> {
    params: QueryParams<'a>,
}

impl<'a> ExecutionLogsQuery<'a> {
    pub fn new(query: Option<&'a str>) -> Self {
        Self {
            params: QueryParams::new(query),
        }
    }

    /// Limit the number of rows returned, clamped to [1, 1000].
    pub fn limit(&self) -> i64 {
        let value = self.params.get_i64("limit", 100);
        value.clamp(1, 1000)
    }
}
