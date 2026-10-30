# Örnek test projesi: JavaFX + Cucumber + TestFX + Allure

raporcu ile çalışan, gerçekten koşan küçük bir referans proje. `testdata/allure/ornek-giris/`
altındaki örnek sonuçlar bu projenin Xvfb altındaki gerçek koşusundan alındı. Kendi test
projene Allure'u eklerken buradaki parçaları kopyala.

## Koşturma

```bash
# Linux (gerçek X11 ekranı, Monocle yok)
xvfb-run -a -s "-screen 0 1920x1080x24" mvn -B test -Dmaven.test.failure.ignore=true
ls target/allure-results
```

CI'da bunu `stajci start` yapar (Xvfb, bellek sınırları, features.txt tag'leri, canlı gönderim).

## Senaryolar ve ürettikleri Allure çıktısı

| Senaryo | Tag | Allure | Neden |
|---------|-----|--------|-------|
| Geçerli bilgilerle giriş | `@DOORS-1001 @smoke` | passed | |
| Hatalı girişlerde mesaj (2 örnek satırı) | `@DOORS-1002` | passed ×2 | Senaryo taslağı: her satır ayrı sonuç |
| Yanlış şifrede özel mesaj | `@DOORS-1003 @JIRA-STAJ-42 @sample-fail` | **failed** ×2 | assertion → ürün hatası örneği |
| Şifremi unuttum bağlantısı | `@DOORS-1004` | **broken** ×2 | olmayan düğüm → test kodu hatası örneği |
| Yavaş açılan karşılama mesajı | `@DOORS-1005 @flaky` | failed → **passed** | rerun: aynı `historyId` ile iki sonuç |
| Beni hatırla seçeneği | `@DOORS-1006 @wip` | (status yok) | tanımsız adım: `status` alanı hiç yazılmaz |

Çıktıda dikkat edilecekler (raporcu ayrıştırıcısı bunlara göre yazıldı):

- `*-result.json` her deneme için ayrı; rerun'da `historyId` aynı kalır.
- `@After` hook'unda `scenario.attach(...)` ile eklenen ekran görüntüsü ve video **result
  dosyasında değil**, ilgili `*-container.json` içindeki `afters[].attachments` altındadır;
  `children` alanı hangi sonuca ait olduğunu söyler.
- `scenario.attach` ile eklenen dosyaların uzantısı yoktur (`...-attachment`); tür json'daki `type`.
- Adım içi `Allure.addAttachment` ekleri `steps[].attachments` altındadır (`...-attachment.txt`).
- DOORS/Jira numaraları tag olarak `labels[name=tag]` içinde gelir.

## Kendi projene kopyalanacaklar

1. **`pom.xml`** — `allure-cucumber7-jvm` bağımlılığı; surefire: `forkCount=1`,
   `argLine` içinde `-Xmx${test.xmx}` ve `-Djava.awt.headless=false`, `rerunFailingTestsCount`.
2. **`src/test/resources/junit-platform.properties`** —
   `cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm`
3. **`src/test/resources/allure.properties`** — `allure.results.directory=target/allure-results`
4. **`Hooks.java`** — `@Before`: `FxToolkit.registerPrimaryStage()` + `setupApplication(...)`;
   `@After`: senaryo kaldıysa pencere bölgesinin robot ile alınan gerçek ekran görüntüsü
   `scenario.attach(png, "image/png", "Ekran görüntüsü")`. Başarılı senaryoda görüntü alınmaz.
5. **`VideoKaydi.java`** — senaryo boyunca ffmpeg (Linux `x11grab`, Windows `gdigrab`), 5 fps
   H.264; yalnızca senaryo kalırsa `video/mp4` olarak eklenir. `STAJ_VIDEO=false` ile kapanır.
6. **`features.txt`** — koşulacak tag ifadeleri; `stajci preflight` kontrol eder.
7. **`AGENTS.md`, `opencode.json`, `.opencode/agents/fix.md`** — self-healing için.
