use axum::http::HeaderMap;
use opentelemetry::propagation::Extractor;

/// Custom header extractor for OpenTelemetry trace propagation
pub(crate) struct HeaderExtractor<'a>(pub &'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
