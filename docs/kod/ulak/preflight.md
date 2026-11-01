# ulak/src/preflight.rs

**Amaç:** SW-1.5 — testlerden önce test projesini denetler; HATA varsa çıkış 1 ve testler başlamaz.

| # | Kontrol | Sonuç |
|---|---------|-------|
| 1 | `pom.xml` var | yoksa HATA (diğerleri yapılmaz) |
| 2 | pom'da `allure-cucumber7-jvm` (ya da `allure-cucumber-jvm`) | yoksa HATA "Allure adaptörü eksik…" |
| 3 | `AllureCucumber7Jvm` pom'da ya da `src/test` altındaki `.properties`/`.java` dosyalarında | yoksa HATA (junit-platform.properties satırı önerilir) |
| 4 | Monocle/headless: pom'da `openjfx-monocle`, `<artifactid>monocle`, `glass.platform=monocle`, `testfx.headless=true`, `<testfx.headless>true`, `prism.order=sw` (küçük harfle) | varsa HATA |
| 5 | pom'da `-Xmx` | yoksa UYARI |
| 6 | surefire varsa `<forkCount>1</forkCount>` | yoksa UYARI |
| 7 | `src/test` altında en az bir senaryo | yoksa HATA |
| 7b | `cucumber.execution.parallel.enabled=true` (properties ya da pom) | HATA (TestFX tek ekran, canlı ekran tek senaryo) |
| 7c | `CanliOlayEklentisi` properties/java/pom içinde | yoksa UYARI (canlı ekranda adım görünmez; `docs/CANLI_EKRAN_KURULUM.md`) |
| 7d | `ULAK_FIX` kapalı değil ve ajan OpenCode ise `.opencode/agents/<ad>.md` var, `ULAK_OZET`/`ULAK_TRIAGE` yazdırıyor, `mode: subagent` değil; `.claude/skills` ve `.opencode/skills` altında her klasörde `name` klasör adıyla aynı, `description` dolu | bozuksa UYARI |
| 8 | `features.txt`: ayrıştırılamazsa HATA; ifadedeki her tag en az bir senaryoda olmalı (yoksa HATA); hiç senaryo seçmiyorsa HATA; dosya yoksa UYARI | |

Çıktı satırları: `ön kontrol: TAMAM|UYARI|HATA  …`, sonda geçti ya da "N hata ile düştü; testler
başlatılmadı".

## Testler
`test_sw_1_5_sample_project_passes`, `…missing_allure_adapter_fails_clearly`,
`…missing_plugin_config_fails`, `…features_txt_unknown_tag_fails`, `…monocle_rejected`.

## Python karşılığı
Saf dosya okuma ve metin arama; `pathlib.Path.rglob`.
