//! stajci ↔ raporcu istek/yanıt tipleri.

use serde::{Deserialize, Serialize};

/// GitLab CI'dan gelen koşum bilgisi (SW-4.3, SW-4.6). stajci bunları CI_* değişkenlerinden doldurur.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CiInfo {
    /// GitLab proje kimliği (CI_PROJECT_ID). Yoksa proje yolu kimlik olarak kullanılır.
    pub project_id: Option<String>,
    /// CI_PROJECT_PATH, örn. `ekip/javafx-testleri`
    pub project_path: String,
    pub project_url: Option<String>,
    pub branch: String,
    pub commit: Option<String>,
    pub pipeline_id: Option<String>,
    pub pipeline_url: Option<String>,
    pub job_id: Option<String>,
    pub job_url: Option<String>,
    /// `linux` / `windows`
    pub os: Option<String>,
}

impl CiInfo {
    /// Sorgu dizesine çevirir (POST /api/ci/runs?...).
    pub fn to_query(&self) -> String {
        let mut parts = Vec::new();
        let mut push = |k: &str, v: &Option<String>| {
            if let Some(v) = v {
                parts.push(format!("{k}={}", enc(v)));
            }
        };
        push("project_id", &self.project_id);
        push("project_path", &Some(self.project_path.clone()));
        push("project_url", &self.project_url);
        push("branch", &Some(self.branch.clone()));
        push("commit", &self.commit);
        push("pipeline_id", &self.pipeline_id);
        push("pipeline_url", &self.pipeline_url);
        push("job_id", &self.job_id);
        push("job_url", &self.job_url);
        push("os", &self.os);
        parts.join("&")
    }
}

fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunRequest {
    #[serde(flatten)]
    pub ci: CiInfo,
    /// dry-run ile bulunan toplam senaryo sayısı (SW-3.1)
    pub total: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCreated {
    pub run_id: i64,
    pub report_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestSummary {
    pub run_id: i64,
    pub accepted_results: usize,
    pub duplicate_results: usize,
    pub containers: usize,
    pub attachments: usize,
    pub report_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishRequest {
    /// `finished` (varsayılan) ya da `interrupted`
    pub status: Option<String>,
}

/// stajci fix → sunucu (SW-7.3)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FixResult {
    pub run_id: i64,
    pub scenario_key: String,
    /// `mr_opened` | `draft_mr` | `no_change` | `rejected` | `error`
    pub outcome: String,
    pub mr_iid: Option<i64>,
    pub mr_url: Option<String>,
    pub branch: Option<String>,
    pub validation_passed: Option<bool>,
    pub summary: Option<String>,
    pub warnings: Vec<String>,
    pub diff: Option<String>,
    /// OpenCode'un triage önerisi (SW-4.5)
    pub triage: Option<TriageSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TriageSuggestion {
    pub category: String,
    pub reason: String,
    pub confidence: f64,
}

/// Sunucu → stajci fix (SW-7.1)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FixCandidate {
    pub scenario_id: i64,
    pub scenario_key: String,
    pub name: String,
    pub feature: Option<String>,
    pub feature_file: Option<String>,
    pub status: String,
    pub target: String,
    pub message: Option<String>,
    pub trace: Option<String>,
    pub steps: Vec<String>,
    pub screenshot_url: Option<String>,
    pub tags: Vec<String>,
    /// Benzer senaryoların daha önce birleştirilmiş düzeltmeleri (SW-7.7)
    pub examples: Vec<FixExample>,
    /// Senaryonun küçük metin ekleri (örn. TestFX sahne ağacı, uygulama logu)
    pub texts: Vec<FixText>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FixText {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FixExample {
    pub scenario: String,
    pub message: Option<String>,
    pub diff: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_4_3_ci_info_query_encoding() {
        let ci = CiInfo {
            project_path: "ekip/javafx testleri".into(),
            branch: "feature/ş".into(),
            ..Default::default()
        };
        let q = ci.to_query();
        assert!(q.contains("project_path=ekip%2Fjavafx%20testleri"));
        assert!(q.contains("branch=feature%2F%C5%9F"));
    }
}
