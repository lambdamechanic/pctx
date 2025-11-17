use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    sync::{Arc, Mutex},
};
use tracing::{
    Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

#[derive(Debug, Serialize)]
struct LogEntry {
    timestamp: DateTime<Utc>,
    level: String,
    target: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<String>,
    #[serde(flatten)]
    fields: serde_json::Map<String, serde_json::Value>,
}

pub(crate) struct JsonlWriter {
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl JsonlWriter {
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;

        Ok(Self {
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }

    fn write_entry(&self, entry: &LogEntry) -> std::io::Result<()> {
        let mut writer = self.writer.lock().unwrap();
        serde_json::to_writer(&mut *writer, entry)?;
        writeln!(&mut *writer)?;
        writer.flush()?;
        Ok(())
    }
}

impl Clone for JsonlWriter {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
        }
    }
}

struct FieldVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
    message: Option<String>,
}

impl FieldVisitor {
    fn new() -> Self {
        Self {
            fields: serde_json::Map::new(),
            message: None,
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(value_str);
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value_str),
            );
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }
}

pub(crate) struct JsonlLayer {
    writer: JsonlWriter,
}

impl JsonlLayer {
    pub(crate) fn new(writer: JsonlWriter) -> Self {
        Self { writer }
    }
}

impl<S> Layer<S> for JsonlLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        let metadata = event.metadata();
        let level = metadata.level();

        if *level == Level::TRACE {
            return;
        }

        let span_name = ctx.event_span(event).map(|span| span.name().to_string());

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            target: metadata.target().to_string(),
            message: visitor.message.unwrap_or_default(),
            span: span_name,
            fields: visitor.fields,
        };

        let _ = self.writer.write_entry(&entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use tempfile::NamedTempFile;

    #[test]
    fn test_jsonl_writer() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let writer = JsonlWriter::new(path).unwrap();

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "test".to_string(),
            message: "test message".to_string(),
            span: None,
            fields: serde_json::Map::new(),
        };

        writer.write_entry(&entry).unwrap();

        // Read and verify
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

        assert_eq!(lines.len(), 1);
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["message"], "test message");
    }
}
