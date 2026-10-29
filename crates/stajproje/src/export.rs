//! SW-4.8: alan seçmeli CSV (Jira dışa aktarma gibi) ve SW-6.4: DOORS dışa aktarma (CSV, DXL).
//!
//! CSV akış halinde yazılır: satırlar veritabanından okundukça parça parça gönderilir (SW-10.6).
//! UTF-8 BOM ile başlar; Excel Türkçe karakterleri doğru açar.

use crate::queries::{search_where, SearchFilter};
use crate::AppState;
use axum::body::Body;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use bytes_chunk::Chunker;
use rusqlite::Connection;

pub const FIELDS: &[(&str, &str)] = &[
    ("proje", "Proje"),
    ("branch", "Branch"),
    ("kosum", "Koşum"),
    ("senaryo", "Senaryo"),
    ("feature", "Feature"),
    ("taglar", "Tag'ler"),
    ("durum", "Durum"),
    ("triage", "Triage kategorisi"),
    ("hata", "Hata mesajı"),
    ("sure", "Süre (ms)"),
    ("doors", "DOORS numarası"),
    ("jira", "Jira issue"),
    ("tarih", "Tarih"),
];

pub fn parse_fields(s: Option<&str>) -> Vec<&'static str> {
    let wanted: Vec<&str> = s.unwrap_or("").split(',').map(str::trim).filter(|x| !x.is_empty()).collect();
    let picked: Vec<&'static str> = wanted
        .iter()
        .filter_map(|w| FIELDS.iter().find(|(k, _)| k == w).map(|(k, _)| *k))
        .collect();
    if picked.is_empty() {
        FIELDS.iter().map(|(k, _)| *k).collect()
    } else {
        picked
    }
}

pub fn csv_cell(v: &str, sep: char) -> String {
    let mut v = v.replace('\0', "");
    // Excel formül enjeksiyonuna karşı
    if v.starts_with(['=', '+', '-', '@']) {
        v.insert(0, '\'');
    }
    if v.contains(sep) || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

mod bytes_chunk {
    /// Satırları ~64 KB'lık parçalar halinde kanala yollar.
    pub struct Chunker {
        buf: String,
        tx: tokio::sync::mpsc::Sender<Result<String, std::io::Error>>,
    }
    impl Chunker {
        pub fn new(tx: tokio::sync::mpsc::Sender<Result<String, std::io::Error>>) -> Self {
            Chunker {
                buf: String::with_capacity(70_000),
                tx,
            }
        }
        pub fn push(&mut self, s: &str) -> bool {
            self.buf.push_str(s);
            if self.buf.len() >= 64 * 1024 {
                return self.flush();
            }
            true
        }
        pub fn flush(&mut self) -> bool {
            if self.buf.is_empty() {
                return true;
            }
            let chunk = std::mem::take(&mut self.buf);
            self.tx.blocking_send(Ok(chunk)).is_ok()
        }
    }
}

fn write_csv(c: &Connection, f: &SearchFilter, fields: &[&str], sep: char, out: &mut Chunker) -> anyhow::Result<()> {
    out.push("\u{feff}");
    let header: Vec<String> = fields
        .iter()
        .map(|k| FIELDS.iter().find(|(x, _)| x == k).unwrap().1.to_string())
        .collect();
    out.push(&(header.iter().map(|h| csv_cell(h, sep)).collect::<Vec<_>>().join(&sep.to_string()) + "\r\n"));
    let (w, p) = search_where(f);
    let mut st = c.prepare(&format!(
        "SELECT p.path, s.branch, s.run_id, s.name, s.feature, s.tags, s.status, s.triage_category, s.message, s.duration_ms,
                s.doors_ids, TRIM(s.jira_keys || ' ' || COALESCE((SELECT GROUP_CONCAT(b.jira_key, ' ') FROM bug_links b WHERE b.scenario_id = s.id), '')),
                s.created_at
         FROM scenarios s JOIN projects p ON p.id = s.project_id WHERE {w} ORDER BY s.run_id DESC, s.id"
    ))?;
    let mut rows = st.query(rusqlite::params_from_iter(p.iter()))?;
    while let Some(r) = rows.next()? {
        let get = |i: usize| -> String {
            match r.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Text(t)) => String::from_utf8_lossy(t).trim().to_string(),
                Ok(rusqlite::types::ValueRef::Integer(n)) => n.to_string(),
                _ => String::new(),
            }
        };
        let mut cells = Vec::with_capacity(fields.len());
        for k in fields {
            let v = match *k {
                "proje" => get(0),
                "branch" => get(1),
                "kosum" => get(2),
                "senaryo" => get(3),
                "feature" => get(4),
                "taglar" => get(5).split_whitespace().map(|t| format!("@{t}")).collect::<Vec<_>>().join(" "),
                "durum" => get(6),
                "triage" => {
                    let t = get(7);
                    if t.is_empty() {
                        t
                    } else {
                        crate::triage::label(&t).to_string()
                    }
                }
                "hata" => get(8).lines().next().unwrap_or("").to_string(),
                "sure" => get(9),
                "doors" => get(10),
                "jira" => {
                    let mut ks: Vec<String> = get(11).split_whitespace().map(String::from).collect();
                    ks.dedup();
                    ks.join(" ")
                }
                "tarih" => get(12),
                _ => String::new(),
            };
            cells.push(csv_cell(&v, sep));
        }
        if !out.push(&(cells.join(&sep.to_string()) + "\r\n")) {
            return Ok(()); // istemci bağlantıyı kapattı
        }
    }
    out.flush();
    Ok(())
}

pub fn stream_csv(st: &AppState, f: SearchFilter, fields: Vec<&'static str>, sep: char, filename: String) -> Response {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<String, std::io::Error>>(4);
    let db = st.db.clone();
    tokio::task::spawn_blocking(move || {
        let tx2 = tx.clone();
        let res = db.read_blocking(|c| {
            let mut ch = Chunker::new(tx);
            write_csv(c, &f, &fields, sep, &mut ch)
        });
        if let Err(e) = res {
            tracing::error!(hata = %e, "CSV yazılamadı");
            let _ = tx2.blocking_send(Err(std::io::Error::other("csv")));
        }
    });
    let stream = futures_util::stream::poll_fn(move |cx| rx.poll_recv(cx));
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}

// ---------------------------------------------------------------------------------------------
// SW-6.4 DOORS

pub struct DoorsRow {
    pub doors_id: String,
    pub scenario: String,
    pub feature: String,
    pub status: String,
    pub jira: String,
    pub date: String,
}

pub fn doors_rows(c: &Connection, run_id: i64) -> anyhow::Result<Vec<DoorsRow>> {
    let mut st = c.prepare(
        "SELECT s.doors_ids, s.name, COALESCE(s.feature, ''), s.status,
                TRIM(s.jira_keys || ' ' || COALESCE((SELECT GROUP_CONCAT(b.jira_key, ' ') FROM bug_links b WHERE b.scenario_id = s.id), '')),
                s.created_at
         FROM scenarios s WHERE s.run_id = ?1 AND s.is_latest = 1 AND TRIM(s.doors_ids) <> '' ORDER BY s.id",
    )?;
    let mut out = Vec::new();
    let mut rows = st.query([run_id])?;
    while let Some(r) = rows.next()? {
        let ids: String = r.get(0)?;
        for id in ids.split_whitespace() {
            out.push(DoorsRow {
                doors_id: id.to_string(),
                scenario: r.get(1)?,
                feature: r.get(2)?,
                status: doors_status(&r.get::<_, String>(3)?).to_string(),
                jira: r.get::<_, String>(4)?.split_whitespace().collect::<Vec<_>>().join(" "),
                date: r.get(5)?,
            });
        }
    }
    Ok(out)
}

/// DOORS tarafında kullanılan sonuç değerleri.
pub fn doors_status(s: &str) -> &'static str {
    match s {
        "passed" => "Geçti",
        "failed" | "broken" => "Kaldı",
        "skipped" => "Atlandı",
        _ => "Koşulmadı",
    }
}

pub fn doors_csv(rows: &[DoorsRow], run_id: i64) -> String {
    let mut s = String::from("\u{feff}DOORS ID,Senaryo,Feature,Sonuç,Jira,Koşum,Tarih\r\n");
    for r in rows {
        s.push_str(
            &[
                r.doors_id.as_str(),
                &r.scenario,
                &r.feature,
                &r.status,
                &r.jira,
                &run_id.to_string(),
                &r.date,
            ]
            .map(|v| csv_cell(v, ','))
            .join(","),
        );
        s.push_str("\r\n");
    }
    s
}

fn dxl_str(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' | '\r' => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// DXL betiği: açık modülde "Object Identifier" (ya da DOORS_ID_ATTR) değeri eşleşen nesnelerin
/// "Test Sonucu", "Test Tarihi" ve "Jira" özniteliklerini günceller.
pub fn doors_dxl(rows: &[DoorsRow], run_id: i64, report_url: &str) -> String {
    let mut s = String::new();
    s.push_str("// stajproje DOORS aktarımı — koşum ");
    s.push_str(&run_id.to_string());
    s.push_str("\n// Kullanım: hedef modülü açın, Tools > Edit DXL ile bu dosyayı çalıştırın.\n");
    s.push_str("// Öznitelik adları modülünüze göre aşağıdan değiştirilebilir.\n\n");
    s.push_str("string idAttr = \"Absolute Number\"\nstring resultAttr = \"Test Sonucu\"\nstring dateAttr = \"Test Tarihi\"\nstring jiraAttr = \"Jira\"\nstring linkAttr = \"Test Raporu\"\n\n");
    s.push_str(&format!("string reportUrl = {}\n", dxl_str(report_url)));
    s.push_str(&format!("int n = {}\n", rows.len()));
    s.push_str("string ids[] = {");
    s.push_str(
        &rows
            .iter()
            .map(|r| dxl_str(r.doors_id.trim_start_matches("DOORS-")))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str("}\nstring results[] = {");
    s.push_str(&rows.iter().map(|r| dxl_str(&r.status)).collect::<Vec<_>>().join(", "));
    s.push_str("}\nstring dates[] = {");
    s.push_str(&rows.iter().map(|r| dxl_str(&r.date)).collect::<Vec<_>>().join(", "));
    s.push_str("}\nstring jiras[] = {");
    s.push_str(&rows.iter().map(|r| dxl_str(&r.jira)).collect::<Vec<_>>().join(", "));
    s.push_str("}\n\n");
    s.push_str(
        r#"Module m = current
if (null m) { ack "Önce hedef modülü açın"; halt }
int updated = 0
int i
for (i = 0; i < n; i++) {
    Object o
    for o in entire m do {
        string v = o.idAttr ""
        if (v == ids[i]) {
            if (canWrite(o.resultAttr)) o.resultAttr = results[i]
            if (canWrite(o.dateAttr)) o.dateAttr = dates[i]
            if (canWrite(o.jiraAttr)) o.jiraAttr = jiras[i]
            if (canWrite(o.linkAttr)) o.linkAttr = reportUrl
            updated++
        }
    }
}
infoBox "stajproje: " updated " nesne güncellendi"
"#,
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_4_8_csv_cell_escaping() {
        assert_eq!(csv_cell("düz", ','), "düz");
        assert_eq!(csv_cell("a,b", ','), "\"a,b\"");
        assert_eq!(csv_cell("a\"b", ','), "\"a\"\"b\"");
        assert_eq!(csv_cell("=1+1", ','), "'=1+1");
        assert_eq!(csv_cell("a;b", ';'), "\"a;b\"");
    }

    #[test]
    fn test_sw_4_8_fields_selected_in_order() {
        assert_eq!(parse_fields(Some("durum,senaryo,olmayan")), vec!["durum", "senaryo"]);
        assert_eq!(parse_fields(None).len(), FIELDS.len());
    }
}
