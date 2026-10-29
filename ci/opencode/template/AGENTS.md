# AGENTS.md — <PROJE ADI> test projesi

Bu dosya OpenCode (ve diğer kod ajanları) için projeyi tanıtır. stajproje self-healing akışı
(`stajci fix`) bu dosyayı ve `.opencode/agent/fix.md` ajanını kullanır.

## Proje

- Test edilen uygulama: <JavaFX uygulamasının adı ve kısa tanımı>
- Testler: Cucumber 7 (Gherkin, Türkçe anahtar kelimeler) + TestFX, JUnit Platform üzerinden.
- Raporlama: Allure (`allure-cucumber7-jvm`), sonuçlar `target/allure-results`.

## Klasörler

| Yol | İçerik |
|-----|--------|
| `src/test/resources/features/` | .feature dosyaları |
| `src/test/java/.../steps/` | adım tanımları (step definitions) |
| `src/test/java/.../Hooks.java` | uygulamayı açan/kapatan, hata anında ekran görüntüsü alan hook'lar |
| `src/main/` | **uygulama kodu — değiştirilmez** |

## Kurallar

- Yalnızca `src/test/` altında değişiklik yap.
- Assertion'ları gevşetme; beklenen davranışı değiştirme.
- Seçiciler: TestFX `#fxId` biçimini tercih et; metne bağlı seçicilerden kaçın.
- Bekleme: `Thread.sleep` yerine `WaitForAsyncUtils` / TestFX koşullu beklemeleri kullan.

## Doğrulama

Tek senaryo koşturmak için:

    mvn -B test -Dcucumber.filter.name="^Senaryo adı$"

Linux'ta gerçek ekran gerekir (Xvfb); stajci bunu kendisi sağlar.
