//! SW-10.7: JSON satır logları.
//!
//! - Her satır bir olay: `zaman`, `seviye`, `olay` (mesaj) ve olayın alanları (`proje`, `branch`,
//!   `pipeline_id`, `run_id`, `senaryo_id`, `sure_ms`, `hata`, ...).
//! - Gizli alanlar (adı token/sifre/password/cookie/authorization içeren) asla yazılmaz.
//! - Metin alanları en fazla 500 karakterdir.
//! - Günlük dosya; en fazla `log_max_days` gün ve toplam `log_max_bytes` (önce dolan sınır).
//! - Son WARN/ERROR kayıtları bellekte tutulur, admin panelinde gösterilir.

use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub const MAX_FIELD_CHARS: usize = 500;
const RECENT_CAP: usize = 200;
const SECRET_KEYS: &[&str] = &["token", "sifre", "şifre", "password", "parola", "cookie", "authorization", "secret"];

#[derive(Clone, Default)]
pub struct RecentLogs(Arc<Mutex<VecDeque<Value>>>);

impl RecentLogs {
    pub fn push(&self, v: Value) {
        let mut q = self.0.lock().unwrap();
        if q.len() >= RECENT_CAP {
            q.pop_front();
        }
        q.push_back(v);
    }

    /// En yeniden eskiye.
    pub fn list(&self) -> Vec<Value> {
        self.0.lock().unwrap().iter().rev().cloned().collect()
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

fn is_secret_key(name: &str) -> bool {
    let n = name.to_lowercase();
    SECRET_KEYS.iter().any(|k| n.contains(k))
}

struct JsonVisitor<'a>(&'a mut Map<String, Value>);

impl JsonVisitor<'_> {
    fn put(&mut self, field: &Field, v: Value) {
        let name = field.name();
        let key = if name == "message" { "olay" } else { name };
        let v = if is_secret_key(key) {
            Value::String("***".into())
        } else {
            match v {
                Value::String(s) => Value::String(truncate(&s, MAX_FIELD_CHARS)),
                other => other,
            }
        };
        self.0.insert(key.to_string(), v);
    }
}

impl Visit for JsonVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.put(field, Value::String(value.to_string()));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.put(field, Value::from(value));
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.put(field, Value::from(value));
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.put(field, Value::from(value));
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.put(field, Value::from(value));
    }
    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.put(field, Value::String(value.to_string()));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.put(field, Value::String(format!("{value:?}")));
    }
}

pub fn event_to_json(event: &Event<'_>) -> Value {
    let mut map = Map::new();
    map.insert(
        "zaman".into(),
        Value::String(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
    );
    map.insert("seviye".into(), Value::String(event.metadata().level().to_string()));
    event.record(&mut JsonVisitor(&mut map));
    if !map.contains_key("olay") {
        map.insert("olay".into(), Value::String(event.metadata().name().to_string()));
    }
    Value::Object(map)
}

type Sink = Box<dyn Write + Send>;

pub struct JsonLayer {
    sinks: Vec<Mutex<Sink>>,
    recent: RecentLogs,
}

impl<S: Subscriber> Layer<S> for JsonLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let v = event_to_json(event);
        if *event.metadata().level() <= Level::WARN {
            self.recent.push(v.clone());
        }
        let mut line = v.to_string();
        line.push('\n');
        for s in &self.sinks {
            if let Ok(mut w) = s.lock() {
                let _ = w.write_all(line.as_bytes());
                let _ = w.flush();
            }
        }
    }
}

pub struct LogHandle {
    pub recent: RecentLogs,
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

/// Global logger kurulumu (sunucu açılışında bir kez).
pub fn init(logs_dir: &Path, max_days: usize, max_bytes: u64, to_stdout: bool) -> anyhow::Result<LogHandle> {
    std::fs::create_dir_all(logs_dir)?;
    enforce_retention(logs_dir, max_days, max_bytes);
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("raporcu")
        .filename_suffix("log")
        .max_log_files(max_days.max(1))
        .build(logs_dir)?;
    let (nb, guard) = tracing_appender::non_blocking(appender);
    let mut sinks: Vec<Mutex<Sink>> = vec![Mutex::new(Box::new(nb))];
    if to_stdout {
        sinks.push(Mutex::new(Box::new(std::io::stdout())));
    }
    let recent = RecentLogs::default();
    let filter = EnvFilter::try_from_env("LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(JsonLayer {
            sinks,
            recent: recent.clone(),
        })
        .try_init()
        .map_err(|e| anyhow::anyhow!("logger kurulamadı: {e}"))?;
    Ok(LogHandle {
        recent,
        _guard: Some(guard),
    })
}

/// Log dosyalarını sınırlar: en fazla `max_files` dosya ve toplam `max_bytes`; en eskiden silinir.
pub fn enforce_retention(dir: &Path, max_files: usize, max_bytes: u64) -> (usize, u64) {
    let Ok(rd) = std::fs::read_dir(dir) else { return (0, 0) };
    let mut files: Vec<(String, u64, std::path::PathBuf)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let meta = e.metadata().ok()?;
            (meta.is_file() && name.starts_with("raporcu")).then(|| (name, meta.len(), e.path()))
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    let (mut removed, mut freed) = (0, 0);
    while !files.is_empty() && (files.len() > max_files || total > max_bytes) {
        let (_, size, path) = files.remove(0);
        if std::fs::remove_file(&path).is_ok() {
            total -= size;
            removed += 1;
            freed += size;
        }
    }
    (removed, freed)
}

/// Testler için: global logger yerine bellekte satır toplayan bir logger kurar (bir kez).
static TEST_RECENT: std::sync::OnceLock<RecentLogs> = std::sync::OnceLock::new();

/// Test logger'ının son WARN/ERROR listesi (testlerdeki AppState bunu kullanır).
pub fn test_recent() -> RecentLogs {
    test_capture();
    TEST_RECENT.get_or_init(RecentLogs::default).clone()
}

pub fn test_capture() -> Arc<Mutex<Vec<String>>> {
    static CAP: std::sync::OnceLock<Arc<Mutex<Vec<String>>>> = std::sync::OnceLock::new();
    CAP.get_or_init(|| {
        let buf = Arc::new(Mutex::new(Vec::new()));
        struct VecSink(Arc<Mutex<Vec<String>>>);
        impl Write for VecSink {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().push(String::from_utf8_lossy(b).trim_end().to_string());
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let layer = JsonLayer {
            sinks: vec![Mutex::new(Box::new(VecSink(buf.clone())))],
            recent: TEST_RECENT.get_or_init(RecentLogs::default).clone(),
        };
        let _ = tracing_subscriber::registry().with(EnvFilter::new("info")).with(layer).try_init();
        buf
    })
    .clone()
}

/// Test yardımcıları: verilen metni içeren log satırları.
pub fn captured_lines(cap: &Arc<Mutex<Vec<String>>>, needle: &str) -> Vec<Value> {
    cap.lock()
        .unwrap()
        .iter()
        .filter(|l| l.contains(needle))
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_10_7_json_fields_secret_and_truncation() {
        let cap = test_capture();
        let long = "x".repeat(2000);
        tracing::warn!(run_id = 987_654_i64, proje = "ekip/p", gitlab_token = "glpat-GIZLI", hata = %long, "log-testi-1");
        let lines = captured_lines(&cap, "log-testi-1");
        assert_eq!(lines.len(), 1);
        let l = &lines[0];
        assert_eq!(l["olay"], "log-testi-1");
        assert_eq!(l["seviye"], "WARN");
        assert_eq!(l["run_id"], 987_654);
        assert!(l["zaman"].as_str().unwrap().ends_with('Z'));
        assert_eq!(l["gitlab_token"], "***");
        assert!(l["hata"].as_str().unwrap().chars().count() <= MAX_FIELD_CHARS + 1);
        assert!(!cap.lock().unwrap().iter().any(|l| l.contains("glpat-GIZLI")));
    }

    #[test]
    fn test_sw_10_7_retention_by_count_and_size() {
        let tmp = tempfile::tempdir().unwrap();
        for d in 1..=20 {
            std::fs::write(tmp.path().join(format!("raporcu.2026-01-{d:02}.log")), vec![b'a'; 100]).unwrap();
        }
        let (removed, _) = enforce_retention(tmp.path(), 14, 1_000_000);
        assert_eq!(removed, 6);
        assert!(!tmp.path().join("raporcu.2026-01-01.log").exists());
        let (removed, _) = enforce_retention(tmp.path(), 14, 550);
        assert_eq!(removed, 9);
        assert!(tmp.path().join("raporcu.2026-01-20.log").exists());
    }

    #[test]
    fn test_sw_10_7_recent_warn_error_kept() {
        let r = RecentLogs::default();
        for i in 0..250 {
            r.push(serde_json::json!({ "i": i }));
        }
        let l = r.list();
        assert_eq!(l.len(), RECENT_CAP);
        assert_eq!(l[0]["i"], 249);
    }
}
