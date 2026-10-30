# AGENTS.md — ornek-test-projesi

OpenCode (raporcu self-healing, `stajci fix`) için proje tanıtımı.

## Proje

- Test edilen uygulama: `com.ornek.app.GirisUygulamasi` — kullanıcı adı + şifre giriş ekranı
  (`#kullaniciAdi`, `#sifre`, `#girisButonu`, sonuç etiketi `#mesaj`).
- Testler: Cucumber 7 + TestFX (`src/test/resources/features/giris.feature`, Türkçe Gherkin).
- Allure: `allure-cucumber7-jvm`, sonuçlar `target/allure-results`.
- Hata anında ekran görüntüsü ve video `Hooks.java` + `VideoKaydi.java` ile alınır.

## Klasörler

| Yol | İçerik |
|-----|--------|
| `src/test/resources/features/giris.feature` | senaryolar |
| `src/test/java/com/ornek/test/GirisAdimlari.java` | adım tanımları |
| `src/test/java/com/ornek/test/Hooks.java` | uygulamayı açma/kapama, ekran görüntüsü |
| `src/main/java/com/ornek/app/` | **uygulama kodu — değiştirilmez** |

## Kurallar

- Yalnızca `src/test/` altında değişiklik yap.
- Assertion'ları gevşetme. "Yanlış şifrede özel mesaj" senaryosu kasıtlı ürün hatası örneğidir
  (`@sample-fail`); düzeltilmez.
- Seçiciler `#fxId` biçimindedir; uygulamadaki gerçek kimlikler yukarıda.

## Doğrulama

    mvn -B test -Dcucumber.filter.name="^Şifremi unuttum bağlantısı$"
