//! allure-results dosya biçimi (allure-java 2.x, allure-cucumber7-jvm).
//!
//! - `*-result.json`: bir senaryo denemesi
//! - `*-container.json`: fixture'lar; Cucumber `@Before/@After` hook'ları ve ekleri burada
//! - `*-attachment[.ext]`: ek dosyaları (json içindeki `source` alanı dosya adıdır)
//!
//! Tüm alanlar isteğe bağlı okunur: gerçek çıktıda örneğin tanımsız adımlı senaryoda `status`
//! hiç yazılmaz (bkz. `testdata/allure/ornek-giris`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Passed,
    Failed,
    Broken,
    Skipped,
    #[default]
    #[serde(other)]
    Unknown,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Passed => "passed",
            Status::Failed => "failed",
            Status::Broken => "broken",
            Status::Skipped => "skipped",
            Status::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Status {
        match s {
            "passed" => Status::Passed,
            "failed" => Status::Failed,
            "broken" => Status::Broken,
            "skipped" => Status::Skipped,
            _ => Status::Unknown,
        }
    }

    /// Başarısız sayılan durumlar (raporda "kaldı").
    pub fn is_failure(&self) -> bool {
        matches!(self, Status::Failed | Status::Broken)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusDetails {
    pub message: Option<String>,
    pub trace: Option<String>,
    pub flaky: bool,
    pub known: bool,
    pub muted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Label {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Link {
    pub name: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Parameter {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Attachment {
    pub name: Option<String>,
    pub source: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Step {
    pub name: String,
    pub status: Option<Status>,
    pub status_details: Option<StatusDetails>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
    pub steps: Vec<Step>,
    pub attachments: Vec<Attachment>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TestResult {
    pub uuid: String,
    pub history_id: Option<String>,
    pub test_case_id: Option<String>,
    pub name: String,
    pub full_name: Option<String>,
    pub description: Option<String>,
    pub status: Option<Status>,
    pub status_details: Option<StatusDetails>,
    pub stage: Option<String>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
    pub labels: Vec<Label>,
    pub links: Vec<Link>,
    pub steps: Vec<Step>,
    pub attachments: Vec<Attachment>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Fixture {
    pub name: String,
    pub status: Option<Status>,
    pub status_details: Option<StatusDetails>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
    pub steps: Vec<Step>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Container {
    pub uuid: String,
    pub children: Vec<String>,
    pub befores: Vec<Fixture>,
    pub afters: Vec<Fixture>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Result,
    Container,
    Attachment,
    Other,
}

/// allure-results içindeki bir dosya adının türü. Yalnızca dosya adına (yol değil) bakar.
pub fn file_kind(file_name: &str) -> FileKind {
    if file_name.ends_with("-result.json") {
        FileKind::Result
    } else if file_name.ends_with("-container.json") {
        FileKind::Container
    } else if file_name.contains("-attachment") {
        FileKind::Attachment
    } else {
        FileKind::Other
    }
}

/// Ek dosya adı güvenli mi: yol ayırıcı, `..`, sürücü harfi içermez.
pub fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.contains(['/', '\\', ':', '\0'])
        && name != "."
        && name != ".."
        && !name.starts_with("..")
}

impl TestResult {
    pub fn status(&self) -> Status {
        self.status.unwrap_or_default()
    }

    /// Senaryonun proje içindeki kalıcı kimliği. Rerun'da aynı kalır.
    pub fn scenario_key(&self) -> String {
        self.history_id
            .clone()
            .or_else(|| self.test_case_id.clone())
            .or_else(|| self.full_name.clone())
            .unwrap_or_else(|| self.name.clone())
    }

    pub fn label(&self, name: &str) -> Option<&str> {
        self.labels.iter().find(|l| l.name == name).map(|l| l.value.as_str())
    }

    pub fn tags(&self) -> Vec<String> {
        self.labels
            .iter()
            .filter(|l| l.name == "tag")
            .map(|l| l.value.trim_start_matches('@').to_string())
            .collect()
    }

    pub fn feature(&self) -> Option<&str> {
        self.label("feature").or_else(|| self.label("suite"))
    }

    pub fn message(&self) -> Option<&str> {
        self.status_details.as_ref().and_then(|d| d.message.as_deref())
    }

    pub fn trace(&self) -> Option<&str> {
        self.status_details.as_ref().and_then(|d| d.trace.as_deref())
    }

    pub fn duration_ms(&self) -> Option<i64> {
        Some(self.stop? - self.start?)
    }

    /// `@DOORS-1234` tag'lerinden DOORS numaraları.
    pub fn doors_ids(&self) -> Vec<String> {
        self.tags().into_iter().filter_map(|t| doors_id(&t)).collect()
    }

    /// `@JIRA-ABC-12` tag'lerinden Jira issue anahtarları (`ABC-12`).
    pub fn jira_keys(&self) -> Vec<String> {
        self.tags().into_iter().filter_map(|t| jira_key(&t)).collect()
    }

    /// Senaryodaki tüm ekler (adımlar dahil, iç içe).
    pub fn all_attachments(&self) -> Vec<&Attachment> {
        let mut out: Vec<&Attachment> = self.attachments.iter().collect();
        fn walk<'a>(steps: &'a [Step], out: &mut Vec<&'a Attachment>) {
            for s in steps {
                out.extend(s.attachments.iter());
                walk(&s.steps, out);
            }
        }
        walk(&self.steps, &mut out);
        out
    }
}

pub fn doors_id(tag: &str) -> Option<String> {
    let t = tag.trim_start_matches('@');
    let rest = t.strip_prefix("DOORS-").or_else(|| t.strip_prefix("doors-"))?;
    (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .then(|| format!("DOORS-{rest}"))
}

pub fn jira_key(tag: &str) -> Option<String> {
    let t = tag.trim_start_matches('@');
    let rest = t.strip_prefix("JIRA-").or_else(|| t.strip_prefix("jira-"))?;
    let (proj, num) = rest.rsplit_once('-')?;
    (!proj.is_empty()
        && proj.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !num.is_empty()
        && num.chars().all(|c| c.is_ascii_digit()))
    .then(|| format!("{}-{}", proj.to_ascii_uppercase(), num))
}

pub fn parse_result(bytes: &[u8]) -> Result<TestResult, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn parse_container(bytes: &[u8]) -> Result<Container, serde_json::Error> {
    serde_json::from_slice(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/allure/ornek-giris")
    }

    fn load() -> (Vec<TestResult>, Vec<Container>) {
        let mut results = Vec::new();
        let mut containers = Vec::new();
        for e in std::fs::read_dir(sample_dir()).unwrap() {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            match file_kind(&name) {
                FileKind::Result => results.push(parse_result(&std::fs::read(&p).unwrap()).unwrap()),
                FileKind::Container => containers.push(parse_container(&std::fs::read(&p).unwrap()).unwrap()),
                _ => {}
            }
        }
        (results, containers)
    }

    #[test]
    fn test_sw_2_7_real_samples_parse() {
        let (results, containers) = load();
        assert_eq!(results.len(), 11);
        assert!(!containers.is_empty());
        let statuses: Vec<Status> = results.iter().map(|r| r.status()).collect();
        for s in [Status::Passed, Status::Failed, Status::Broken, Status::Unknown] {
            assert!(statuses.contains(&s), "{s:?} örneği yok");
        }
        // Rerun: aynı historyId ile birden fazla sonuç
        let flaky: Vec<_> = results.iter().filter(|r| r.name == "Yavaş açılan karşılama mesajı").collect();
        assert_eq!(flaky.len(), 2);
        assert_eq!(flaky[0].scenario_key(), flaky[1].scenario_key());
        // @After ekleri container'da
        let after_shots = containers
            .iter()
            .flat_map(|c| c.afters.iter())
            .flat_map(|f| f.attachments.iter())
            .filter(|a| a.kind.as_deref() == Some("image/png"))
            .count();
        assert!(after_shots >= 4);
        // Adım ekleri
        assert!(results.iter().any(|r| !r.all_attachments().is_empty()));
    }

    #[test]
    fn test_sw_2_7_doors_and_jira_tags() {
        let (results, _) = load();
        let r = results.iter().find(|r| r.name == "Yanlış şifrede özel mesaj").unwrap();
        assert_eq!(r.doors_ids(), vec!["DOORS-1003"]);
        assert_eq!(r.jira_keys(), vec!["STAJ-42"]);
        assert_eq!(r.feature(), Some("Giriş ekranı"));
        assert!(r.message().unwrap().contains("Şifre hatalı"));
    }

    #[test]
    fn test_sw_2_1_safe_file_names() {
        assert!(is_safe_file_name("abc-attachment.png"));
        assert!(!is_safe_file_name("../x"));
        assert!(!is_safe_file_name("a/b"));
        assert!(!is_safe_file_name("a\\b"));
        assert!(!is_safe_file_name("C:x"));
        assert!(!is_safe_file_name(".."));
    }
}
