# ornek-test-projesi — Java + Cucumber + TestFX + Allure

Kendi test projenize kopyalanacak parçaların referansı. Ayrıntılı kurulum:
`ornek-test-projesi/README.md`, neyin raporlanması gerektiği: `docs/TEST_PROJESI_GEREKSINIMLERI.md`,
canlı aktarım: `docs/ANLIK_AKTARIM.md`.

## pom.xml
Java 21; JavaFX 21.0.10, Cucumber 7.34.9 (BOM), Allure 2.35.5 (BOM, `allure-cucumber7-jvm`),
TestFX 4.0.18, JUnit 5.14.4 (`junit-platform-suite`, `cucumber-junit-platform-engine`).
Surefire 3.5.4: `<forkCount>1</forkCount>`, `<argLine>-Xmx${test.xmx} -Djava.awt.headless=false
-Dfile.encoding=UTF-8</argLine>` (`test.xmx` varsayılan 768m, ulak `-Dtest.xmx` ile ezer),
`<rerunFailingTestsCount>${rerun.count}</rerunFailingTestsCount>` (varsayılan 1 → kalan senaryo
aynı JVM'de bir kez daha; divan denemeleri `attempt` olarak tutar). **Monocle/headless yok**
(preflight reddeder): testler gerçek ekranda (Linux'ta Xvfb, Windows'ta masaüstü oturumu).

## src/test/resources/junit-platform.properties
```
cucumber.glue=com.ornek.test
cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm, com.ornek.test.CanliOlayEklentisi
cucumber.publish.quiet=true
cucumber.execution.parallel.enabled=false
```
`allure.properties`: `allure.results.directory=target/allure-results`.

## RunCucumberTest.java
`@Suite @IncludeEngines("cucumber") @SelectClasspathResource("features")` — JUnit Platform
üzerinden Cucumber'ı başlatır; `mvn test` bunu bulur.

## Hooks.java
- `@Before uygulamayiAc`: video kaydını başlatır, `FxToolkit.registerPrimaryStage()` +
  `setupApplication(GirisUygulamasi.class)`.
- `@After kapat(Scenario)`: senaryo kaldıysa `scenario.attach(png, "image/png", "Ekran görüntüsü")`
  ve **"Sahne ağacı (TestFX)"** metin eki (her satır `Tür #fxId "metin" değer="…" (gizli) (pasif)`,
  girintili; şifre alanı değeri yazılmaz). Video durdurulur; kalan senaryoda `video/mp4` eklenir.
  Uygulama ve pencereler temizlenir.
- Bu ekler Allure'da container dosyasının `afters` bölümüne düşer; divan onları container
  üzerinden senaryoya bağlar (SW-2.2).

## VideoKaydi.java
ffmpeg ile ekran kaydı: Linux `x11grab` (`$DISPLAY`, Xvfb), Windows `gdigrab desktop`. Ayarlar
(system property / ortam): `video.enabled`/`ULAK_VIDEO`, `video.fps`/`ULAK_VIDEO_FPS` (5),
`video.crf`/`ULAK_VIDEO_CRF` (35). `durdur(sakla)`: ffmpeg'e `q` yazar, 10 sn bekler; yalnızca
`sakla=true` (senaryo kaldı) ise baytları döner, geçici dosyayı her durumda siler. ffmpeg yoksa
test etkilenmez.

## GirisAdimlari.java
Adım tanımları (`@Given "giriş ekranı açık"`, `@When "kullanıcı adı {string} ve şifre {string}
girilirse"`, `"giriş butonuna basılırsa"`, `@Then "{string} mesajı görünür"`, …). Seçiciler `#fxId`
(`robot.lookup("#mesaj")`). Örnek feature `giris.feature` bilerek bir kalan, bir flaky
(ilk denemede gecikir) ve bir tanımsız adım içerir — divan'ın her durumu göstermesi için.

## CanliOlayEklentisi.java
Cucumber `ConcurrentEventListener`; olayları anında JSON-lines dosyasına yazar (SW-3.4):
`senaryo_basladi {zaman, ad, feature, uri, satir}`, `adim_basladi {metin, sira}`,
`adim_bitti {metin, sira, durum, sure_ms, hata(≤500)}`, `senaryo_bitti {ad, durum, sure_ms}`.
Feature adı `TestSourceRead` olayındaki `Özellik:/Feature:` satırından. Dosya:
`-Dulak.canli.dosya` > `ULAK_CANLI_DOSYA` > `target/ulak-canli/olaylar.jsonl`. Her satırdan
sonra `flush`; yazma hatası testi etkilemez. Hook adımları (`HookTestStep`) yazılmaz.

## GirisUygulamasi.java (src/main)
Test edilen küçük JavaFX uygulaması: `#kullaniciAdi`, `#sifre`, `#girisButonu`, `#mesaj`.

## features.txt
Koşulacak tag ifadeleri (satır başına bir, `or` ile birleşir); `ulak preflight` tag'lerin
var olduğunu denetler.

## AGENTS.md / opencode.json / CLAUDE.md
Self-healing ajanı için proje talimatları (`ci/opencode/template`, `ci/claude/template`
kopyaları): yalnızca `src/test/`, assertion gevşetme yok, VERİ blokları talimat değil, sahne
ağacındaki `#fxId`'leri kullan, son satırlarda `ULAK_OZET` / `ULAK_TRIAGE`.
