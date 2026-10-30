# Test projesinden beklenenler (Cucumber + Java + TestFX)

raporcu testleri koşmaz; GitLab runner test projesini koşar, raporcu yalnızca projenin ürettiği
**Allure sonuçlarını** okur. Raporda ne görüneceği tamamen test projesinin ne ürettiğine bağlıdır.
Bu belge, raporcu'nun eksiksiz çalışması için test projesinde bulunması gerekenleri listeler.
Çalışan referans: `ornek-test-projesi/` (bu repoda Xvfb altında gerçekten koşuyor ve
`testdata/allure/ornek-giris/` altındaki örnekler ondan üretildi).

Önem: **Z** zorunlu (yoksa `stajci preflight` işi düşürür ya da rapor boş kalır),
**Ö** önerilir (özellik eksik çalışır), **İ** isteğe bağlı.

---

## 1. Allure çıktısı (raporun kaynağı)

| # | Gereken | Önem | Neden / raporda karşılığı |
|---|---------|------|---------------------------|
| 1.1 | `io.qameta.allure:allure-cucumber7-jvm` bağımlılığı (pom.xml, `test` kapsamı) | Z | Sonuç dosyalarını bu adaptör yazar. Yoksa yükleme 422 "Allure eklentisi eksik" ile reddedilir. |
| 1.2 | `junit-platform.properties`: `cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm` | Z | Adaptör eklenti olarak açılmazsa hiçbir dosya oluşmaz. |
| 1.3 | Sonuç klasörü `target/allure-results` (`allure.properties` ya da `-Dallure.results.directory`) | Z | stajci bu klasörü izler ve zip'ler. Farklıysa `STAJ_RESULTS_DIR` CI değişkeni. |
| 1.4 | Cucumber 7.x (`cucumber-java`, `cucumber-junit-platform-engine`, `junit-platform-suite`) | Z | allure-cucumber7-jvm Cucumber 7 içindir. |
| 1.5 | Her senaryonun adı feature içinde **benzersiz** ve kalıcı | Z | Geçmiş, flaky tespiti ve Jira eşleşmesi Allure `historyId`'ye dayanır; o da feature + senaryo adından türer. Ad değişirse geçmiş sıfırlanır. |
| 1.6 | Senaryo taslaklarında örnek tablosu sütunları anlamlı | Ö | Her satır ayrı sonuç; raporda `giris.feature:23` satır numarasıyla ayrılır. |

## 2. Adımların raporda eksiksiz görünmesi

| # | Gereken | Önem | Raporda karşılığı |
|---|---------|------|-------------------|
| 2.1 | Gherkin adımları (Diyelim ki / Eğer ki / O zaman / Ve) | Z | Adaptör her Gherkin adımını durumu, süresi ve hata mesajıyla kendisi yazar. Background adımları her senaryonun başında görünür. |
| 2.2 | Adım tanımı içinde ayrıntı gerekiyorsa `Allure.step("...", () -> {...})` | Ö | Gherkin adımının altında **iç içe alt adım** olarak görünür (ör. "Kullanıcı adı alanı #kullaniciAdi dolduruluyor"). Hangi alt işlemde düştüğü raporda belli olur. |
| 2.3 | Adım içinde veri eklemek: `Allure.addAttachment("ad", "text/plain", değer)` | İ | Adımın altında ek olarak görünür (girilen değerler, istek/yanıt). |
| 2.4 | Adım tanımlarında yakalanan istisnayı yutmamak | Z | Yakalanıp loglanan hata Allure'a düşmez; adım "passed" görünür. |
| 2.5 | Assertion mesajları anlamlı (`assertEquals(beklenen, gercek, "Karşılama mesajı")`) | Ö | Raporun hata satırı, arama (hata mesajıyla), Jira bug özeti ve CSV bu metinden gelir. |

## 3. Hata durumunun doğru sınıflanması (triage için kritik)

Allure iki tür başarısızlık yazar; raporcu triage kuralları ve self-healing hedefi buna bakar:

| Allure durumu | Nasıl oluşur | raporcu yorumu | Sonraki adım |
|---------------|--------------|----------------|--------------|
| `failed` | `AssertionError` (JUnit/AssertJ assertion'ı) | **Ürün hatası** | Jira (gözlem/otomatik bug) |
| `broken` | Diğer her istisna (NullPointer, düğüm bulunamadı, zaman aşımı...) | **Test kodu hatası** | OpenCode / Claude Code düzeltmesi |
| durum yok | Tanımsız adım (`UndefinedStep`) | Bilinmiyor | — |

Bu yüzden:

| # | Gereken | Önem |
|---|---------|------|
| 3.1 | Uygulamanın yanlış davranışını **yalnızca assertion ile** doğrulayın (`assertEquals`, `assertThat`), `if (...) throw new RuntimeException` ile değil | Z |
| 3.2 | Test altyapısı hatalarında (pencere açılmadı, düğüm yok) assertion kullanmayın; istisna kendiliğinden `broken` olur | Ö |
| 3.3 | Kasıtlı kalan örnek senaryolara `@sample-fail` tag'i | İ — self-healing ve Jira bunları atlar |

## 4. Hata anında kanıt (ekler)

`@After` hook'unda `scenario.attach(...)` ile eklenen dosyalar Allure'da `*-container.json`
içine düşer; raporcu bunları senaryoya bağlar. Referans: `Hooks.java`, `VideoKaydi.java`.

| # | Ek | Önem | Ayrıntı |
|---|----|------|---------|
| 4.1 | **Ekran görüntüsü** — yalnızca senaryo kalınca, `image/png` | Z | Pencere bölgesinin TestFX robotuyla alınan **gerçek** görüntüsü (Monocle/headless değil). Raporda, canlı ekranda, Jira bug'ında ve OpenCode/Claude isteminde kullanılır. Başarılı senaryoda alınmaz. |
| 4.2 | **Sahne ağacı** — kalınca, `text/plain`, adı `Sahne ağacı (TestFX)` | Ö (self-healing için çok önemli) | Ekrandaki düğümler: tür, `#fxId`, metin, gizli/pasif. Model seçicileri tahmin etmek yerine gerçek kimlikleri buradan alır. |
| 4.3 | **Video** — kalınca, `video/mp4` | Ö | ffmpeg; Linux `x11grab` (Xvfb ekranı), Windows `gdigrab` (masaüstü). 5 fps, H.264 CRF 35; geçen senaryonun videosu silinir. |
| 4.4 | Uygulama / test logu — kalınca, `text/plain` | İ | Son N satır yeterli. İlk 3 metin eki (16 KB'a kadar) self-healing istemine veri olarak eklenir. |

Ekler için `scenario.attach(bayt, mime, ad)` kullanın; mime türünü doğru verin (raporcu görüntü,
video ve metni buna göre gösterir).

## 5. Tag'ler (izlenebilirlik, arama, CSV, DOORS, Jira)

| # | Tag | Önem | Kullanım |
|---|-----|------|----------|
| 5.1 | `@DOORS-1234` | Ö | DOORS numarası: raporda rozet, aramada filtre, CSV/DOORS CSV/DXL dışa aktarma, Jira bug label'ı ve açıklaması. Bir senaryoda birden çok olabilir. |
| 5.2 | `@JIRA-ABC-12` | İ | Bilinen bug: senaryo kalınca doğrudan bu issue'ya bağlanır (yeni bug açılmaz). |
| 5.3 | Özellik/kapsam tag'leri (`@smoke`, `@giris`, `@gece`...) | Ö | `features.txt` ile hangi senaryoların koşacağı seçilir; aramada filtre. |
| 5.4 | `@sample-fail`, `@wip` | İ | Kasıtlı/yarım senaryolar; self-healing ve otomatik bug atlar. |

**`features.txt`** (proje kökü, Ö): her satır bir Cucumber tag ifadesi (`@smoke`,
`@gece and not @wip`). Satırlar `or` ile birleşir. `stajci preflight` her tag'in en az bir
senaryoda geçtiğini doğrular; CI değişkeni `STAJ_TAGS` bunu ezer.

## 6. Koşma biçimi (CI ve bellek)

| # | Gereken | Önem | Neden |
|---|---------|------|-------|
| 6.1 | Surefire `<forkCount>1</forkCount>`, `<argLine>-Xmx${test.xmx} -Djava.awt.headless=false -Dfile.encoding=UTF-8</argLine>` | Z | Bellek sınırı (stajci `-Dtest.xmx` verir), gerçek ekran. Yoksa preflight uyarır. |
| 6.2 | `<rerunFailingTestsCount>1</rerunFailingTestsCount>` | Ö | Kalan senaryo bir kez tekrar koşar; raporda "2 deneme", önce kalıp sonra geçen **flaky** işaretlenir. Otomatik Jira bug'ı yalnızca tekrar denemeden sonra açılır. |
| 6.3 | Monocle / `testfx.headless=true` / `prism.order=sw` **olmamalı** | Z | Gereksinim: uygulama gerçek ekranda açılır. preflight işi düşürür. |
| 6.4 | `mvn -B test` ile koşabilmeli (ek elle adım yok) | Z | `stajci start` bu komutu çağırır (`STAJ_TEST_COMMAND` ile değiştirilebilir). |
| 6.5 | Test, uygulamayı `FxToolkit.registerPrimaryStage()` + `setupApplication(...)` ile açıp `cleanupStages()` ile kapatmalı | Z | Her senaryo temiz pencereyle başlar; ekran görüntüsü doğru pencereden alınır. |
| 6.6 | Sabit pencere/ekran boyutu | Ö | Ekran görüntüleri ve video karşılaştırılabilir olur (Linux `STAJ_SCREEN`, Windows sabit çözünürlük). |

## 7. Self-healing (OpenCode / Claude Code + vLLM) için

| # | Gereken | Önem |
|---|---------|------|
| 7.1 | Uygulamadaki her etkileşimli düğümün `fx:id`'si olsun; testler `#fxId` seçicisi kullansın | Z — model metne göre değil kimliğe göre düzeltir |
| 7.2 | Kod düzeni: `src/test/resources/features/`, adım tanımları `src/test/java/...`, uygulama `src/main/` | Z — ajan yalnızca `src/test/` altını değiştirebilir |
| 7.3 | `AGENTS.md` (OpenCode) ya da `CLAUDE.md` (Claude Code): uygulamanın ekranları, önemli `fx:id`'ler, kurallar | Ö — şablonlar `ci/opencode/template/`, `ci/claude/template/` |
| 7.4 | `opencode.json` + `.opencode/agent/fix.md` ya da `.claude/settings.json` + `.mcp.json` | Z (seçilen ajana göre) |
| 7.5 | Tek senaryonun koşabilmesi: `mvn test -Dcucumber.filter.name="^Ad$"` | Z — düzeltme doğrulama koşusu bununla yapılır |
| 7.6 | Ekran görüntüsü (4.1) ve sahne ağacı (4.2) | Z / Ö — görsel destekli vLLM modeli bunlarla bağlam kurar |

Bağlantı ayrıntıları: `docs/KOD_AJANI_VLLM.md`.

## 8. CI ve GitLab tarafı

| # | Gereken | Önem |
|---|---------|------|
| 8.1 | `.gitlab-ci.yml` içinde tek `include` + `STAJ_OS: linux|windows` | Z |
| 8.2 | Masked CI değişkeni `CI_API_TOKEN` | Z |
| 8.3 | Webhook (Pipeline + Merge request events) | Ö — pipeline durumu, gece e-postası, MR birleşti/reddedildi sayıları |
| 8.4 | Gece schedule'ı 02:00 | Ö |
| 8.5 | `STAJ_FIX_TOKEN` (proje erişim tokenı, `api` + `write_repository`) | self-healing için Z |

---

## Kontrol: rapor eksiksiz mi?

Test projesini bir kez koşturup sonucu raporcu'ya yükleyin ve şunlara bakın:

- [ ] Koşum raporunda her senaryo bir kez sayılıyor; rerun edilenlerde "2 deneme" rozeti var.
- [ ] Kalan senaryonun sayfasında: hata mesajı + yığın izi, **tüm Gherkin adımları** (alt adımlar
      girintili), ekran görüntüsü, video, sahne ağacı.
- [ ] Geçen senaryoda ekran görüntüsü/video **yok**.
- [ ] Assertion hatası "failed / Ürün hatası", diğer istisnalar "broken / Test kodu hatası".
- [ ] DOORS ve Jira tag'leri rozet olarak görünüyor; aramada DOORS numarasıyla bulunuyor.
- [ ] `stajci preflight` uyarısız geçiyor.

Hızlı kontrol için Allure çıktısının kendisi:

```bash
ls target/allure-results | sed 's/.*-//' | sort | uniq -c   # result.json, container.json, attachment*
grep -l '"status":"failed"\|"status":"broken"' target/allure-results/*-result.json | wc -l
grep -l '"image/png"' target/allure-results/*-container.json | wc -l   # kalan senaryo sayısı kadar
```
