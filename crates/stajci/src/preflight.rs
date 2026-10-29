//! SW-1.5: ön kontrol. Testlerden önce CI içinde çalışır; eksik varsa iş açık bir mesajla düşer.
//!
//! Kontroller:
//! 1. `pom.xml` var
//! 2. Allure adaptörü (`allure-cucumber7-jvm`) bağımlılığı var
//! 3. Allure Cucumber eklentisi ayarlı (`AllureCucumber7Jvm`, properties ya da pom içinde)
//! 4. Headless/Monocle kullanılmıyor (SW-1.2, SW-1.6)
//! 5. Surefire bellek sınırı (`-Xmx`) ve tek fork (SW-1.7) — uyarı
//! 6. `features.txt` tag'leri geçerli ve en az bir senaryoda kullanılıyor

use crate::ctx::say;
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

    let lower = pom.to_lowercase();
    if ["openjfx-monocle", "<artifactid>monocle", "glass.platform=monocle", "testfx.headless=true", "<testfx.headless>true", "prism.order=sw"]
        .iter()
        .any(|n| lower.contains(n))
    {
        r.errors.push("Headless/Monocle ayarı bulundu. Testler gerçek ekranda koşmalı (Linux: Xvfb, Windows: masaüstü oturumu); Monocle kaldırılmalı.".into());
    } else {
        r.ok.push("Headless/Monocle yok".into());
    }

    if !pom.contains("-Xmx") {
        r.warnings.push("surefire argLine içinde -Xmx yok; test JVM belleği sınırsız. Örnek: <argLine>-Xmx${test.xmx}</argLine>".into());
    }
    if pom.contains("maven-surefire-plugin") && !pom.contains("<forkCount>1</forkCount>") {
        r.warnings.push("surefire forkCount 1 değil; bellek için tek fork önerilir (SW-1.7)".into());
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
                r.errors.push(format!("features.txt ({}) hiçbir senaryo seçmiyor", lines.join(" | ")));
            } else {
                r.ok.push(format!("features.txt: {} satır, {n} senaryo seçiliyor", lines.len()));
            }
        }
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
        let pom = std::fs::read_to_string(p.join("pom.xml")).unwrap().replace("allure-cucumber7-jvm", "baska-bir-sey");
        std::fs::write(p.join("pom.xml"), pom).unwrap();
        let r = check(&p);
        assert!(r.errors.iter().any(|e| e.contains("Allure adaptörü eksik")));
        assert_eq!(run(&p), 1);
    }

    #[test]
    fn test_sw_1_5_missing_plugin_config_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        std::fs::write(p.join("src/test/resources/junit-platform.properties"), "cucumber.glue=com.ornek.test\n").unwrap();
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
    fn test_sw_1_5_monocle_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample(tmp.path());
        let pom = std::fs::read_to_string(p.join("pom.xml")).unwrap().replace("</dependencies>", "<dependency><artifactId>openjfx-monocle</artifactId></dependency></dependencies>");
        std::fs::write(p.join("pom.xml"), pom).unwrap();
        assert!(check(&p).errors.iter().any(|e| e.contains("Monocle")));
    }
}
