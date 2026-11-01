//! SW-1.5: ön kontrol. Testlerden önce CI içinde çalışır; eksik varsa iş açık bir mesajla düşer.
//!
//! Kontroller:
//! 1. `pom.xml` var
//! 2. Allure adaptörü (`allure-cucumber7-jvm`) bağımlılığı var
//! 3. Allure Cucumber eklentisi ayarlı (`AllureCucumber7Jvm`, properties ya da pom içinde)
//! 4. Headless/Monocle kullanılmıyor (SW-1.2, SW-1.6)
//! 5. Surefire bellek sınırı (`-Xmx`) ve tek fork (SW-1.7) — uyarı
//! 6. `features.txt` tag'leri geçerli ve en az bir senaryoda kullanılıyor
//! 7. Cucumber paralel koşumu kapalı (TestFX tek ekran; canlı ekran tek senaryo varsayar) — hata
//! 8. Canlı olay eklentisi (`CanliOlayEklentisi`) ekli — yoksa uyarı (canlı ekranda adım görünmez)
//! 9. Self-healing açıksa OpenCode ajanı ve skill'ler `ulak fix` sözleşmesine uyuyor — uyarı

use crate::ctx::{env, env_or, say};
use crate::gherkin;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub ok: Vec<String>,
}

fn grep_tree(dir: &Path, exts: &[&str], needle: &str) -> bool {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let n = e.file_name().to_string_lossy().to_string();
                if n != "target" && !n.starts_with('.') {
                    stack.push(p);
                }
            } else if exts.iter().any(|x| p.to_string_lossy().ends_with(x)) {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if s.contains(needle) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Ağaçtaki dosyalardan herhangi birinin içeriği `f`'yi sağlıyor mu.
fn grep_lines(dir: &Path, exts: &[&str], f: &dyn Fn(&str) -> bool) -> bool {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let n = e.file_name().to_string_lossy().to_string();
                if n != "target" && !n.starts_with('.') {
                    stack.push(p);
                }
            } else if exts.iter().any(|x| p.to_string_lossy().ends_with(x)) && std::fs::read_to_string(&p).is_ok_and(|s| f(&s)) {
                return true;
            }
        }
    }
    false
}

/// Skill: klasör adıyla aynı `name`, dolu `description` (OpenCode ve Claude Code aynı kuralı ister).
fn skill_sorunu(ad: &str, metin: &str) -> Option<String> {
    let alan = |k: &str| {
        metin
            .lines()
            .take_while(|l| !l.is_empty() || l.starts_with('-'))
            .find_map(|l| l.strip_prefix(k).and_then(|x| x.strip_prefix(':')))
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let uygun = !ad.is_empty()
        && ad
            .split('-')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    if !uygun {
        return Some(format!("skill klasörü `{ad}`: yalnızca küçük harf, rakam ve tek tire olmalı"));
    }
    if !metin.starts_with("---") {
        return Some(format!("skill `{ad}`: SKILL.md `---` ile başlayan name/description başlığı ister"));
    }
    if alan("name").as_deref() != Some(ad) {
        return Some(format!("skill `{ad}`: `name` klasör adıyla aynı olmalı"));
    }
    if alan("description").is_none() {
        return Some(format!("skill `{ad}`: `description` boş; ajan skill'i buna bakarak seçer"));
    }
    None
}

/// Ajan dosyası ve skill'ler bozuksa `ulak fix` gece sessizce işe yaramaz; burada söylenir.
fn kod_ajani_kontrol(project: &Path, ajan: &str, r: &mut Report) {
    let yol = project.join(".opencode").join("agents").join(format!("{ajan}.md"));
    match std::fs::read_to_string(&yol) {
        Err(_) => r.warnings.push(format!(
            "OpenCode ajanı yok: .opencode/agents/{ajan}.md. Self-healing kullanılmayacaksa ULAK_FIX=off yapın."
        )),
        Ok(s) => {
            if !s.contains("ULAK_TRIAGE") || !s.contains("ULAK_OZET") {
                r.warnings.push(format!(
                    ".opencode/agents/{ajan}.md son satırlarda ULAK_OZET ve ULAK_TRIAGE yazdırmalı; yoksa divan ajanın kararını göremez"
                ));
            }
            if s.lines().any(|l| l.replace(' ', "") == "mode:subagent") {
                r.warnings.push(format!(
                    ".opencode/agents/{ajan}.md `mode: subagent` olamaz; `opencode run --agent` için primary olmalı"
                ));
            }
            r.ok.push(format!("OpenCode ajanı: {ajan}"));
        }
    }
    let mut adet = 0;
    for kok in [".claude", ".opencode"] {
        let Ok(rd) = std::fs::read_dir(project.join(kok).join("skills")) else {
            continue;
        };
        for e in rd.flatten().filter(|e| e.path().is_dir()) {
            let ad = e.file_name().to_string_lossy().to_string();
            adet += 1;
            match std::fs::read_to_string(e.path().join("SKILL.md")) {
                Err(_) => r.warnings.push(format!("skill `{ad}`: SKILL.md yok")),
                Ok(s) => r.warnings.extend(skill_sorunu(&ad, &s.replace("\r\n", "\n"))),
            }
        }
    }
    if adet > 0 {
        r.ok.push(format!("{adet} skill bulundu"));
    }
}

pub fn check(project: &Path) -> Report {
    let mut r = Report::default();
    let pom_path = project.join("pom.xml");
    let Ok(pom) = std::fs::read_to_string(&pom_path) else {
        r.errors.push(format!("pom.xml bulunamadı: {}", pom_path.display()));
        return r;
    };
    r.ok.push("pom.xml var".into());

    if pom.contains("allure-cucumber7-jvm") || pom.contains("allure-cucumber-jvm") {
        r.ok.push("Allure adaptörü (allure-cucumber7-jvm) var".into());
    } else {
        r.errors.push(
            "Allure adaptörü eksik: pom.xml içinde io.qameta.allure:allure-cucumber7-jvm bağımlılığı yok. \
             Örnek: ornek-test-projesi/pom.xml"
                .into(),
        );
    }

    let plugin = "AllureCucumber7Jvm";
    let test_dir = project.join("src").join("test");
    if pom.contains(plugin) || grep_tree(&test_dir, &[".properties", ".java"], plugin) {
        r.ok.push("Cucumber Allure eklentisi ayarlı".into());
    } else {
        r.errors.push(
            "Cucumber Allure eklentisi ayarlı değil: src/test/resources/junit-platform.properties içine \
             `cucumber.plugin=io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm` ekleyin."
                .into(),
        );
    }

    // Paralel koşum: properties'te ya da pom'da (configurationParameters) açık mı
    let parallel = |t: &str| {
        t.lines().any(|l| {
            let l = l.trim().replace(' ', "");
            !l.starts_with('#') && l.contains("cucumber.execution.parallel.enabled") && l.contains("true")
        })
    };
    if parallel(&pom) || grep_lines(&test_dir, &[".properties"], &parallel) {
        r.errors.push(
            "Cucumber paralel koşumu açık (cucumber.execution.parallel.enabled=true): TestFX aynı ekranda \
             senaryoları sırayla koşmalı; kapatın (false)."
                .into(),
        );
    }
    if pom.contains("CanliOlayEklentisi") || grep_tree(&test_dir, &[".properties", ".java"], "CanliOlayEklentisi") {
        r.ok.push("Canlı olay eklentisi ekli".into());
    } else {
        r.warnings.push(
            "Canlı olay eklentisi (CanliOlayEklentisi) yok: canlı ekranda şu an koşan senaryo/adım görünmez, \
             sonuçlar senaryo bitince gelir. Kurulum: docs/CANLI_EKRAN_KURULUM.md"
                .into(),
        );
    }

    let lower = pom.to_lowercase();
    if [
        "openjfx-monocle",
        "<artifactid>monocle",
        "glass.platform=monocle",
        "testfx.headless=true",
        "<testfx.headless>true",
        "prism.order=sw",
    ]
    .iter()
    .any(|n| lower.contains(n))
    {
        r.errors.push("Headless/Monocle ayarı bulundu. Testler gerçek ekranda koşmalı (Linux: Xvfb, Windows: masaüstü oturumu); Monocle kaldırılmalı.".into());
    } else {
        r.ok.push("Headless/Monocle yok".into());
    }

    if !pom.contains("-Xmx") {
        r.warnings
            .push("surefire argLine içinde -Xmx yok; test JVM belleği sınırsız. Örnek: <argLine>-Xmx${test.xmx}</argLine>".into());
    }
    if pom.contains("maven-surefire-plugin") && !pom.contains("<forkCount>1</forkCount>") {
        r.warnings
            .push("surefire forkCount 1 değil; bellek için tek fork önerilir (SW-1.7)".into());
    }

    let plan_all = gherkin::plan(project, None).unwrap_or_default();
    if plan_all.is_empty() {
        r.errors.push("src/test altında hiç .feature senaryosu bulunamadı".into());
    }
    match gherkin::read_features_txt(&project.join("features.txt")) {
        Err(e) => r.errors.push(format!("features.txt geçersiz: {e}")),
        Ok(None) => r.warnings.push("features.txt yok; tüm senaryolar koşulacak".into()),
        Ok(Some((lines, expr))) => {
            let mut all_tags = std::collections::BTreeSet::new();
            for p in &plan_all {
                all_tags.extend(p.tags.iter().cloned());
            }
            for t in expr.tags() {
                if !all_tags.contains(&t) {
                    r.errors.push(format!("features.txt içindeki @{t} tag'i hiçbir senaryoda yok"));
                }
            }
            let n = plan_all.iter().filter(|p| expr.eval(&p.tags)).count();
            if n == 0 {
                r.errors
                    .push(format!("features.txt ({}) hiçbir senaryo seçmiyor", lines.join(" | ")));
            } else {
                r.ok.push(format!("features.txt: {} satır, {n} senaryo seçiliyor", lines.len()));
            }
        }
    }
    if env("ULAK_FIX").as_deref() != Some("off") && env_or("ULAK_FIX_AGENT", "opencode") == "opencode" {
        kod_ajani_kontrol(project, &env_or("ULAK_OPENCODE_AGENT", "fix"), &mut r);
    }
    r
}

pub fn run(project: &Path) -> i32 {
    let r = check(project);
    for o in &r.ok {
        say(format!("ön kontrol: TAMAM  {o}"));
    }
    for w in &r.warnings {
        say(format!("ön kontrol: UYARI  {w}"));
    }
    for e in &r.errors {
        say(format!("ön kontrol: HATA   {e}"));
    }
    if r.errors.is_empty() {
        say("ön kontrol geçti");
        0
    } else {
        say(format!("ön kontrol {} hata ile düştü; testler başlatılmadı", r.errors.len()));
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_7_2_ajan_ve_skill_sozlesmesi_denetlenir() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path();
        let mut r = Report::default();
        kod_ajani_kontrol(p, "ozel", &mut r);
        assert!(r.warnings.iter().any(|w| w.contains("agents/ozel.md")));

        std::fs::create_dir_all(p.join(".opencode/agents")).unwrap();
        std::fs::write(p.join(".opencode/agents/ozel.md"), "---\nmode: subagent\n---\nDüzelt.\n").unwrap();
        for (ad, metin) in [
            ("iyi-skill", "---\nname: iyi-skill\ndescription: Seçici hatalarında yükle.\n---\n"),
            ("yanlis-ad", "---\nname: baska\ndescription: x\n---\n"),
            ("bos-aciklama", "---\nname: bos-aciklama\ndescription:\n---\n"),
            ("Buyuk_Harf", "---\nname: Buyuk_Harf\ndescription: x\n---\n"),
        ] {
            std::fs::create_dir_all(p.join(".claude/skills").join(ad)).unwrap();
            std::fs::write(p.join(".claude/skills").join(ad).join("SKILL.md"), metin).unwrap();
        }
        let mut r = Report::default();
        kod_ajani_kontrol(p, "ozel", &mut r);
        let w = r.warnings.join("\n");
        assert!(w.contains("ULAK_TRIAGE") && w.contains("subagent"));
        assert!(w.contains("`yanlis-ad`") && w.contains("`bos-aciklama`") && w.contains("`Buyuk_Harf`"));
        assert!(!w.contains("iyi-skill"));
    }

    fn copy_dir(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap().flatten() {
            let n = e.file_name();
            if n == "target" {
                continue;
            }
            if e.path().is_dir() {
                copy_dir(&e.path(), &dst.join(&n));
            } else {
                std::fs::copy(e.path(), dst.join(&n)).unwrap();
            }
        }
    }

    fn sample(tmp: &Path) -> std::path::PathBuf {
        let p = tmp.join("proje");
        copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ornek-test-projesi"), &p);
        p
    }

    #[test]
    fn test_sw_1_5_sample_project_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        let r = check(&p);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
    }

    #[test]
    fn test_sw_1_5_missing_allure_adapter_fails_clearly() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        let pom = std::fs::read_to_string(p.join("pom.xml"))
            .unwrap()
            .replace("allure-cucumber7-jvm", "baska-bir-sey");
        std::fs::write(p.join("pom.xml"), pom).unwrap();
        let r = check(&p);
        assert!(r.errors.iter().any(|e| e.contains("Allure adaptörü eksik")));
        assert_eq!(run(&p), 1);
    }

    #[test]
    fn test_sw_1_5_missing_plugin_config_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        std::fs::write(
            p.join("src/test/resources/junit-platform.properties"),
            "cucumber.glue=com.ornek.test\n",
        )
        .unwrap();
        assert!(check(&p).errors.iter().any(|e| e.contains("eklentisi ayarlı değil")));
    }

    #[test]
    fn test_sw_1_5_features_txt_unknown_tag_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        std::fs::write(p.join("features.txt"), "@smoke\n@olmayan_tag\n").unwrap();
        let r = check(&p);
        assert!(r.errors.iter().any(|e| e.contains("@olmayan_tag")), "{:?}", r.errors);
        std::fs::write(p.join("features.txt"), "smoke\n").unwrap();
        assert!(check(&p).errors.iter().any(|e| e.contains("features.txt geçersiz")));
    }

    #[test]
    fn test_sw_1_5_parallel_rejected_and_live_plugin_warned() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        let f = p.join("src/test/resources/junit-platform.properties");
        let props = std::fs::read_to_string(&f).unwrap();
        assert!(check(&p).warnings.iter().all(|w| !w.contains("CanliOlayEklentisi")));
        std::fs::write(
            &f,
            props.replace(
                "cucumber.execution.parallel.enabled=false",
                "cucumber.execution.parallel.enabled = true",
            ),
        )
        .unwrap();
        assert!(check(&p).errors.iter().any(|e| e.contains("paralel")));
        std::fs::write(&f, props.replace(", com.ornek.test.CanliOlayEklentisi", "")).unwrap();
        std::fs::remove_file(p.join("src/test/java/com/ornek/test/CanliOlayEklentisi.java")).unwrap();
        let r = check(&p);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(r.warnings.iter().any(|w| w.contains("CanliOlayEklentisi")));
    }

    #[test]
    fn test_sw_1_5_monocle_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        let pom = std::fs::read_to_string(p.join("pom.xml")).unwrap().replace(
            "</dependencies>",
            "<dependency><artifactId>openjfx-monocle</artifactId></dependency></dependencies>",
        );
        std::fs::write(p.join("pom.xml"), pom).unwrap();
        assert!(check(&p).errors.iter().any(|e| e.contains("Monocle")));
    }
}
