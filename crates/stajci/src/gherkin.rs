//! .feature dosyalarından senaryo sayımı (SW-3.1 dry-run) ve Cucumber tag ifadeleri.
//!
//! JVM açmadan sayar; Senaryo taslağı her örnek satırı için bir senaryo sayılır. İngilizce ve
//! Türkçe anahtar kelimeler desteklenir.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub name: String,
    pub feature: String,
    pub file: PathBuf,
    pub tags: BTreeSet<String>,
}

const FEATURE: &[&str] = &["Feature:", "Özellik:", "Ability:", "Business Need:"];
const RULE: &[&str] = &["Rule:", "Kural:"];
const BACKGROUND: &[&str] = &["Background:", "Geçmiş:"];
const OUTLINE: &[&str] = &["Scenario Outline:", "Scenario Template:", "Senaryo taslağı:", "Senaryo Taslağı:"];
const SCENARIO: &[&str] = &["Scenario:", "Example:", "Senaryo:", "Örnek:"];
const EXAMPLES: &[&str] = &["Examples:", "Scenarios:", "Örnekler:"];

fn strip<'a>(line: &'a str, kws: &[&str]) -> Option<&'a str> {
    kws.iter().find_map(|k| line.strip_prefix(k)).map(str::trim)
}

fn parse_tags(line: &str) -> Vec<String> {
    line.split('#')
        .next()
        .unwrap_or("")
        .split_whitespace()
        .filter(|t| t.starts_with('@'))
        .map(|t| t.trim_start_matches('@').to_string())
        .collect()
}

pub fn parse_feature(text: &str, file: &Path) -> Vec<Planned> {
    let mut out = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut feature_tags: BTreeSet<String> = BTreeSet::new();
    let mut rule_tags: BTreeSet<String> = BTreeSet::new();
    let mut feature = String::new();
    // açık senaryo taslağı: (ad, tag'ler)
    let mut outline: Option<(String, BTreeSet<String>)> = None;
    // örnekler bloğu: (tag'ler, başlık görüldü mü)
    let mut examples: Option<(BTreeSet<String>, bool)> = None;
    let mut in_doc = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("\"\"\"") || line.starts_with("```") {
            in_doc = !in_doc;
            continue;
        }
        if in_doc || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('@') {
            pending.extend(parse_tags(line));
            continue;
        }
        let take = |p: &mut Vec<String>| -> BTreeSet<String> { p.drain(..).collect() };
        if let Some(name) = strip(line, FEATURE) {
            feature = name.to_string();
            feature_tags = take(&mut pending);
            outline = None;
            examples = None;
        } else if strip(line, RULE).is_some() {
            rule_tags = take(&mut pending);
            outline = None;
            examples = None;
        } else if strip(line, BACKGROUND).is_some() {
            pending.clear();
            outline = None;
            examples = None;
        } else if let Some(name) = strip(line, OUTLINE) {
            let mut tags = feature_tags.clone();
            tags.extend(rule_tags.iter().cloned());
            tags.extend(take(&mut pending));
            outline = Some((name.to_string(), tags));
            examples = None;
        } else if let Some(name) = strip(line, SCENARIO) {
            let mut tags = feature_tags.clone();
            tags.extend(rule_tags.iter().cloned());
            tags.extend(take(&mut pending));
            out.push(Planned { name: name.to_string(), feature: feature.clone(), file: file.to_path_buf(), tags });
            outline = None;
            examples = None;
        } else if strip(line, EXAMPLES).is_some() {
            let t = take(&mut pending);
            examples = outline.as_ref().map(|_| (t, false));
        } else if line.starts_with('|') {
            if let (Some((name, otags)), Some((etags, header_seen))) = (&outline, &mut examples) {
                if !*header_seen {
                    *header_seen = true;
                } else {
                    let mut tags = otags.clone();
                    tags.extend(etags.iter().cloned());
                    out.push(Planned { name: name.clone(), feature: feature.clone(), file: file.to_path_buf(), tags });
                }
            }
        }
    }
    out
}

pub fn feature_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if name != "target" && !name.starts_with('.') && name != "node_modules" {
                    stack.push(p);
                }
            } else if name.ends_with(".feature") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

pub fn plan(project_root: &Path, expr: Option<&TagExpr>) -> anyhow::Result<Vec<Planned>> {
    let mut all = Vec::new();
    for f in feature_files(&project_root.join("src").join("test")) {
        let text = std::fs::read_to_string(&f)?;
        let rel = f.strip_prefix(project_root).unwrap_or(&f).to_path_buf();
        all.extend(parse_feature(&text, &rel));
    }
    Ok(all.into_iter().filter(|p| expr.is_none_or(|e| e.eval(&p.tags))).collect())
}

/// Cucumber tag ifadesi: `@a and not (@b or @c)`
#[derive(Debug, Clone, PartialEq)]
pub enum TagExpr {
    Tag(String),
    Not(Box<TagExpr>),
    And(Box<TagExpr>, Box<TagExpr>),
    Or(Box<TagExpr>, Box<TagExpr>),
}

impl TagExpr {
    pub fn parse(s: &str) -> anyhow::Result<TagExpr> {
        let spaced = s.replace('(', " ( ").replace(')', " ) ");
        let toks: Vec<&str> = spaced.split_whitespace().collect();
        anyhow::ensure!(!toks.is_empty(), "boş tag ifadesi");
        let mut p = Parser { toks, pos: 0 };
        let e = p.or()?;
        anyhow::ensure!(p.pos == p.toks.len(), "tag ifadesinde fazla parça: {}", p.toks[p.pos..].join(" "));
        Ok(e)
    }

    pub fn eval(&self, tags: &BTreeSet<String>) -> bool {
        match self {
            TagExpr::Tag(t) => tags.contains(t),
            TagExpr::Not(e) => !e.eval(tags),
            TagExpr::And(a, b) => a.eval(tags) && b.eval(tags),
            TagExpr::Or(a, b) => a.eval(tags) || b.eval(tags),
        }
    }

    pub fn tags(&self) -> Vec<String> {
        match self {
            TagExpr::Tag(t) => vec![t.clone()],
            TagExpr::Not(e) => e.tags(),
            TagExpr::And(a, b) | TagExpr::Or(a, b) => {
                let mut v = a.tags();
                v.extend(b.tags());
                v
            }
        }
    }
}

struct Parser<'a> {
    toks: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.toks.get(self.pos).copied()
    }
    fn or(&mut self) -> anyhow::Result<TagExpr> {
        let mut l = self.and()?;
        while self.peek() == Some("or") {
            self.pos += 1;
            l = TagExpr::Or(Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }
    fn and(&mut self) -> anyhow::Result<TagExpr> {
        let mut l = self.not()?;
        while self.peek() == Some("and") {
            self.pos += 1;
            l = TagExpr::And(Box::new(l), Box::new(self.not()?));
        }
        Ok(l)
    }
    fn not(&mut self) -> anyhow::Result<TagExpr> {
        if self.peek() == Some("not") {
            self.pos += 1;
            return Ok(TagExpr::Not(Box::new(self.not()?)));
        }
        self.atom()
    }
    fn atom(&mut self) -> anyhow::Result<TagExpr> {
        match self.peek() {
            Some("(") => {
                self.pos += 1;
                let e = self.or()?;
                anyhow::ensure!(self.peek() == Some(")"), "kapanmayan parantez");
                self.pos += 1;
                Ok(e)
            }
            Some(t) if t.starts_with('@') && t.len() > 1 => {
                self.pos += 1;
                Ok(TagExpr::Tag(t[1..].to_string()))
            }
            Some(t) => anyhow::bail!("geçersiz tag: {t} (tag'ler @ ile başlar)"),
            None => anyhow::bail!("tag ifadesi eksik bitti"),
        }
    }
}

/// features.txt: her satır bir tag ifadesi; boş ve `#` satırları yok sayılır. Satırlar `or` ile birleşir.
pub fn read_features_txt(path: &Path) -> anyhow::Result<Option<(Vec<String>, TagExpr)>> {
    let Ok(text) = std::fs::read_to_string(path) else { return Ok(None) };
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return Ok(None);
    }
    let joined = lines.iter().map(|l| format!("({l})")).collect::<Vec<_>>().join(" or ");
    Ok(Some((lines, TagExpr::parse(&joined)?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ornek-test-projesi")
    }

    #[test]
    fn test_sw_3_1_counts_scenarios_and_outline_rows() {
        let all = plan(&ornek(), None).unwrap();
        // 5 senaryo + 2 örnek satırı = 7 (Allure örneğiyle aynı)
        assert_eq!(all.len(), 7);
        assert_eq!(all.iter().filter(|p| p.name == "Hatalı girişlerde mesaj").count(), 2);
        assert!(all.iter().all(|p| p.feature == "Giriş ekranı"));
        assert!(all.iter().all(|p| p.tags.contains("giris")));
    }

    #[test]
    fn test_sw_3_1_tag_filter() {
        let e = TagExpr::parse("@giris and not (@wip or @sample-fail)").unwrap();
        assert_eq!(plan(&ornek(), Some(&e)).unwrap().len(), 5);
        let e = TagExpr::parse("@smoke").unwrap();
        assert_eq!(plan(&ornek(), Some(&e)).unwrap().len(), 1);
    }

    #[test]
    fn test_sw_1_5_tag_expression_errors() {
        assert!(TagExpr::parse("smoke").is_err());
        assert!(TagExpr::parse("@a and").is_err());
        assert!(TagExpr::parse("(@a").is_err());
        assert!(TagExpr::parse("@a or @b and not @c").is_ok());
    }

    #[test]
    fn test_sw_3_1_examples_tags_and_rules() {
        let f = "@f\nFeature: X\n  Rule: R\n  @o\n  Scenario Outline: O\n    Given a <x>\n    @e1\n    Examples:\n      | x |\n      | 1 |\n      | 2 |\n    @e2\n    Examples:\n      | x |\n      | 3 |\n  Scenario: S\n    \"\"\"\n    Scenario: docstring içinde sayılmaz\n    \"\"\"\n";
        let p = parse_feature(f, Path::new("x.feature"));
        assert_eq!(p.len(), 4);
        assert!(p[2].tags.contains("e2") && p[2].tags.contains("o") && p[2].tags.contains("f"));
        assert_eq!(p[3].name, "S");
    }
}
