# Allure test verisi (SW-2.7)

Ayrıştırıcı ve yükleme testleri uydurma değil gerçek `allure-results` çıktısıyla yazılır.

- `ornek-giris/` — `ornek-test-projesi/`nin Xvfb altında gerçek koşusu (allure-cucumber7-jvm 2.35,
  Cucumber 7.34, TestFX 4.0.18, JavaFX 21): 7 senaryo, 11 deneme (rerun), passed / failed /
  broken / durumsuz (tanımsız adım), @After ekran görüntüleri (PNG), videolar (MP4), adım ekleri.

Gerçek test projesi Allure ile ayarlanınca oradan alınan örnekler `testdata/allure/<proje-adı>/`
altına eklenmeli (geçen, kalan, broken, rerun edilmiş ve ekli senaryolar içersin).
