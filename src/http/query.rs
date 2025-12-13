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
