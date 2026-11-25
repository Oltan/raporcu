---
name: cucumber-adim
description: Cucumber 7 (Java) adım tanımı ve feature dosyası kuralları. Undefined/ambiguous adım, yanlış eşleşen adım ifadesi, hook ya da adım sınıfı değişikliği gerektiğinde yükle.
---

# Cucumber adım tanımları

## Yapı
- Feature dosyaları: `src/test/resources/features/*.feature` (dil satırı `# language: tr` olabilir:
  `Özellik`, `Senaryo`, `Senaryo taslağı`, `Örnekler`, `Diyelim ki`, `Eğer ki`, `O zaman`, `Ve`).
- Adım sınıfları: `src/test/java/**/` (glue paketi `junit-platform.properties` içindeki `cucumber.glue`).
- `Hooks.java`: uygulamayı açar/kapatır, hata anında ekran görüntüsü + sahne ağacı + video ekler.
  **Hook'lara dokunma**; raporlama bunlara bağlı.

## Adım ifadeleri
- Cucumber Expressions kullanılır: `@When("kullanıcı adı {string} ve şifre {string} girilirse")`.
  Parametre türleri: `{string}`, `{int}`, `{float}`, `{word}`.
- Anotasyon dili feature dilinden bağımsızdır (`io.cucumber.java.en.When` Türkçe feature ile çalışır);
  eşleşme yalnızca ifade metnine bakar (anahtar kelime hariç).
- **Undefined** adım: önce mevcut adımlarda aynı işi yapan bir ifade ara ve feature'daki metni
  ona uydur ya da yeni bir adım yaz. Yeni adım mevcut yardımcıları (`robot`, sayfa nesneleri) kullansın.
- **Ambiguous** adım: iki ifade aynı metni eşliyor; daha özel olanı bırak, diğerini daralt.
- Aynı ifadeyi iki sınıfta tanımlama.

## Senaryo değişikliği sınırları
- Feature dosyasındaki **beklenen değerleri** (`O zaman "..." mesajı görünür`, `Örnekler` tablosundaki
  beklenen sütunlar) değiştirme; bunlar gereksinimdir. Uygulama başka değer gösteriyorsa → `urun`.
- Tag'leri (`@DOORS-…`, `@JIRA-…`, `@smoke`) silme ya da değiştirme; DOORS ve Jira bağlantısı bunlarla kurulur.
- Adım metnindeki yazım hatası/eski metin düzeltilebilir (adım tanımıyla eşleşmesi için).
