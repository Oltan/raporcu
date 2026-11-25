# Anlık aktarım: test koşarken sonuçlar divan'a nasıl gelir


> Kısa kurulum (yalnızca test projesine eklenecekler, Java kodu dahil): [`CANLI_EKRAN_KURULUM.md`](CANLI_EKRAN_KURULUM.md)

Bu belge bir Java + Cucumber 7 + TestFX test projesinin GitLab runner'da koşarken sonuçlarının
divan'ın canlı ekranına nasıl aktığını uçtan uca anlatır. Kısaca: **iki kanal** var.

| Kanal | Kim yazar | Ne zaman | Ne taşır | divan'da |
|-------|-----------|----------|----------|------------|
| **Canlı olaylar** | `CanliOlayEklentisi` (Cucumber eklentisi, test projesinde) | Anında: senaryo/adım başladığı ve bittiği an | Senaryo adı, feature, adım metni, adım durumu, hata | "Şu an koşuyor" kartı, adım listesi, biten senaryonun sonucu |
| **Allure sonuçları** | `allure-cucumber7-jvm` | Senaryo **bitince** | Tam sonuç: adımlar, alt adımlar, hata + yığın izi, ekran görüntüsü, video, sahne ağacı | Rapor, geçmiş, triage, Jira, CSV — kalıcı kayıt |

Canlı olaylar geçicidir (sunucu belleğinde tutulur). Kalıcı ve eksiksiz kayıt her zaman Allure
sonuçlarıdır. Eklenti olmasa da sistem çalışır; yalnızca "şu an hangi adımda" bilgisi tahmine
döner.

---

## 1. Akış

```
GitLab runner (test makinesi)                                          divan sunucusu
───────────────────────────────────────────────────────────────────    ─────────────────────────────
ulak start
 ├─ .feature dosyalarını sayar (tag süzgeciyle) ──── POST /api/ci/runs/start ──▶ koşum "çalışıyor",
 │                                                  {expected:[ad, feature]}     beklenen senaryolar
 ├─ ulak watch'ı ayrı süreç olarak başlatır
 └─ xvfb-run mvn -B test -Dallure.results.directory=…  -Dulak.canli.dosya=…
        │
        │  JVM (surefire, tek fork)
        │   Cucumber ──▶ CanliOlayEklentisi ──▶ target/ulak-canli/olaylar.jsonl   (anında, satır satır)
        │            └─▶ AllureCucumber7Jvm ──▶ target/allure-results/*.json, *-attachment*  (senaryo bitince)
        │
ulak watch  (saniyede bir, ULAK_WATCH_INTERVAL_MS)
 ├─ olaylar.jsonl: yeni tam satırlar ──────────────── POST /api/ci/runs/{id}/events ──▶ bellekte "şu an"
 └─ allure-results: yeni dosyalar (ekler önce) ────── POST /api/ci/runs/{id}/results?name=… ─▶ SQLite
                                                                                       │
                                                         WebSocket /ws/runs/{id} ◀─────┘ (değişince
                                                         tarayıcı: /runs/{id}/live        anlık görüntü)
mvn biter → ulak start: "stop" dosyası, watch son dosyaları gönderip çıkar
report işi: ulak upload (toplu zip, tekrarlar sayılmaz) → ulak finish
```

## 2. Zaman çizelgesi: tek senaryo

Örnek: `Senaryo: Yanlış şifrede özel mesaj` (kalan senaryo). Süreler `ornek-test-projesi`nin
gerçek koşusundan.

| t (sn) | Test JVM'inde | Yazılan | Canlı ekranda (≤1–2 sn sonra) |
|--------|---------------|---------|-------------------------------|
| 0.0 | Cucumber senaryoyu başlatır (`TestCaseStarted`) | olaylar.jsonl: `senaryo_basladi` | Kart: "şu an koşuyor: Yanlış şifrede özel mesaj", "Hazırlık: @Before" |
| 0.0–3.5 | `@Before`: `FxToolkit` uygulamayı açar, ffmpeg kaydı başlar | — | Süre sayacı ilerler |
| 3.5 | Adım 1 başlar / biter | `adim_basladi`, `adim_bitti PASSED` | Adım listesinde yeşil 1. adım |
| 3.5–4.8 | Adım 2: TestFX robotu alanlara yazar | `adim_basladi` … `adim_bitti` | "Eğer ki kullanıcı adı … koşuyor…" sonra yeşil |
| 5.2 | Adım 4: `assertEquals` kalır | `adim_bitti FAILED` + hata | Kırmızı adım, altında `expected: <Şifre hatalı> but was: …` |
| 5.2–5.6 | `@After`: ekran görüntüsü, sahne ağacı, video eklenir | `*-attachment` dosyaları | — |
| 5.6 | Cucumber senaryoyu bitirir | `senaryo_bitti FAILED`; Allure `*-container.json`, `*-result.json` | Listede "kaldı"; ~1 sn sonra Allure dosyası gelince hata mesajı + ekran görüntüsü küçük resmi |

Allure sonuç dosyası senaryo bitmeden **yazılmaz**; bu yüzden adım düzeyindeki canlılık yalnızca
eklentiyle mümkündür. Ölçülen gecikme (prova, SW-3.2): senaryo bitişinden panelde görünmesine
en kötü ~1–2 sn (gereksinim ≤ 5 sn).

## 3. Test projesinde gerekenler (Java + Cucumber + TestFX)

Hepsinin çalışan örneği `ornek-test-projesi/`dedir.

### 3.1 Allure (zorunlu)

`pom.xml`: `io.qameta.allure:allure-cucumber7-jvm` (test kapsamı). Ayrıntı:
`docs/TEST_PROJESI_GEREKSINIMLERI.md`.

### 3.2 Canlı olay eklentisi (önerilir)

1. `ornek-test-projesi/src/test/java/com/ornek/test/CanliOlayEklentisi.java` dosyasını kendi
   test paketine kopyala (paket adını değiştir). Bağımlılığı yok; yalnızca `cucumber-java`
   içindeki `io.cucumber.plugin` API'sini kullanır.
2. `src/test/resources/junit-platform.properties`:

   ```properties
   cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm, com.sirket.test.CanliOlayEklentisi
   ```

3. Bitti. Dosya yolunu `ulak start` `-Dulak.canli.dosya=…` ile verir; elle koşturunca
   `target/ulak-canli/olaylar.jsonl`'e yazar.

Eklentinin davranışı:

- `TestSourceRead`: feature dosyasının metninden `Özellik:` / `Feature:` satırını okuyup feature
  adını saklar (Cucumber olaylarında feature adı yoktur).
- `TestCaseStarted` → `senaryo_basladi`; `TestStepStarted` (yalnızca Gherkin adımları, hook'lar
  değil) → `adim_basladi`; `TestStepFinished` → `adim_bitti` (durum, süre, hata mesajının ilk 500
  karakteri); `TestCaseFinished` → `senaryo_bitti`.
- Her satırı yazar yazmaz `flush` eder. Yazma hatası yalnızca stderr'e not düşer, testi
  **etkilemez**.
- Paralel koşum kapalı varsayılır (`cucumber.execution.parallel.enabled=false`); TestFX zaten tek
  ekranda tek senaryo koşturur.

### 3.3 Surefire

`ulak start` Maven'a `-Dallure.results.directory` ve `-Dulak.canli.dosya` verir; surefire
bunları forklanan test JVM'ine sistem özelliği olarak geçirir. `forkCount=1` olmalı (bellek
için zaten öyle). `rerunFailingTestsCount` açıksa rerun'da aynı senaryo ikinci kez
`senaryo_basladi` üretir; canlı ekranda "(2. deneme)" yazar, son durum geçerli olur.

## 4. Olay biçimi (JSON lines)

Her satır bir JSON nesnesi. `zaman` test makinesinin saatiyle epoch milisaniye; sunucu bunu yalnızca
bilgi olarak taşır, süreleri kendi saatiyle hesaplar (saat farkı sorun olmaz).

```json
{"tur":"senaryo_basladi","zaman":1793340892175,"ad":"Geçerli bilgilerle giriş","feature":"Giriş ekranı","uri":"classpath:features/giris.feature","satir":10}
{"tur":"adim_basladi","zaman":1793340895673,"metin":"Diyelim ki giriş ekranı açık","sira":1}
{"tur":"adim_bitti","zaman":1793340895710,"metin":"Diyelim ki giriş ekranı açık","sira":1,"durum":"PASSED","sure_ms":37,"hata":null}
{"tur":"senaryo_bitti","zaman":1793340899910,"ad":"Geçerli bilgilerle giriş","durum":"PASSED","sure_ms":7735}
```

`durum` değerleri Cucumber'ınkilerdir: `PASSED`, `FAILED`, `SKIPPED`, `PENDING`, `UNDEFINED`,
`AMBIGUOUS`. Gerçek bir koşunun tamamı: `testdata/canli/ornek-giris-olaylar.jsonl`.

## 5. ulak watch

- Test işinden **ayrı süreç**; çökerse test etkilenmez (iş sonunda toplu zip yine gider).
- Her turda (varsayılan 1 sn):
  1. **Olaylar:** `olaylar.jsonl`'i kaldığı bayttan okur, yalnızca `\n` ile biten tam satırları
     JSON dizisi olarak `POST /api/ci/runs/{id}/events`'e gönderir. Gönderemezse konumu
     ilerletmez, sonraki turda tekrar dener.
  2. **Allure dosyaları:** `allure-results`'ta yeni dosyaları bulur; önce ekler, sonra
     container/result json'ları. Yazımı süren dosyayı göndermez (boyut iki tur aynı kalmalı ya da
     300 ms'den eski olmalı). Her dosya ayrı `POST /api/ci/runs/{id}/results?name=<dosya>`.
     Gönderilenler `.ulak/sent.txt`'e yazılır.
- Sunucuya ulaşamazsa artan aralıklarla (1 → 30 sn) bekler. `stop` dosyası görülünce son turu
  yapıp çıkar; gönderilemeyenler report işindeki toplu yüklemeyle gider.
- Çıktısı `.ulak/watch.log` (iş artifact'inde).

## 6. divan tarafı

| Uç | Ne yapar |
|----|----------|
| `POST /api/ci/runs/start` | Koşumu `running` açar, beklenen senaryoları (ad + feature, sırayla) kaydeder |
| `POST /api/ci/runs/{id}/events` | Olayları bellekteki "şu an" durumuna uygular, koşumun zaman aşımı sayacını tazeler, WebSocket'e haber verir. Koşum bitmişse 202 döner, olayları yok sayar |
| `POST /api/ci/runs/{id}/results?name=` | Tek Allure dosyası: result/container json'u SQLite'a işler (aynı uuid ikinci kez sayılmaz), eki diske yazar, container eklerini senaryoya bağlar |
| `GET /api/runs/{id}/live` | Canlı ekranın gördüğü anlık görüntü (WebSocket'teki ile aynı JSON) |
| `WS /ws/runs/{id}` | Bağlanınca bir görüntü, sonra her değişiklikte yenisi (250 ms içinde gelenler birleştirilir) |

Canlı ekrandaki her beklenen senaryonun durumu sırayla şöyle belirlenir:

1. Allure sonuç dosyası geldiyse: onun durumu (passed / failed / broken / …), hata mesajı,
   ekran görüntüsü.
2. Gelmediyse ama `senaryo_bitti` olayı geldiyse: olaydaki durum (`PASSED` → geçti, `FAILED` →
   kaldı). Rerun'da aynı senaryonun **son** bitişi geçerlidir.
3. Koşan senaryo olaysa: "koşuyor" (kesin bilgi). Eklenti yoksa: sonucu olmayan ilk beklenen
   senaryo "koşuyor" sayılır (tahmin).
4. Geri kalanlar "bekliyor".

"Şu an koşuyor" kartı: senaryo adı, deneme no, feature ve `dosya:satır`, geçen süre, biten
adımlar (durum + süre + hata), koşan adım ya da "Hazırlık: @Before".

Koşum bitince (`finish` ya da zaman aşımı) bellekteki durum silinir; ekran kalıcı sonuçlarla
kalır. Sunucu koşum sırasında yeniden başlarsa yalnızca "şu an" bilgisi kaybolur; bir sonraki
senaryo başlayınca yeniden kurulur. Allure sonuçları kaybolmaz.

## 7. Ayarlar

| Değişken | Nerede | Varsayılan | Açıklama |
|----------|--------|-----------|----------|
| `ULAK_SERVER` | CI değişkeni | `http://127.0.0.1:8080` | divan ayrı makinedeyse onun adresi (HTTP yeterli) |
| `ULAK_WATCH_INTERVAL_MS` | CI | 1000 | İzleme turu |
| `ULAK_WATCH` | CI | açık | `off`: canlı gönderim yok, yalnızca iş sonunda toplu |
| `ULAK_CANLI_DOSYA` | CI | `target/ulak-canli/olaylar.jsonl` | Olay dosyası |
| `ULAK_WATCH_DRAIN_SECS` | CI | 30 | Testler bitince izleyicinin son dosyaları göndermesi için süre |
| `RUN_STALL_TIMEOUT_SECS` | divan `.env` | 900 | Hiç olay/sonuç gelmezse koşum bu süre sonra `interrupted` |

## 8. Sorun giderme

| Belirti | Bakılacak yer |
|---------|---------------|
| Koşum açıldı ama hiçbir şey gelmiyor | `.ulak/watch.log`; `ULAK_SERVER` adresine runner'dan `curl $ULAK_SERVER/api/health` |
| Sonuçlar geliyor, "şu an koşuyor" kartı yok | `target/ulak-canli/olaylar.jsonl` oluşuyor mu? `cucumber.plugin` listesinde eklenti var mı? |
| Feature adı boş | Feature dosyasının ilk anahtar satırı `Özellik:` / `Feature:` mı? |
| Senaryolar "bekliyor"da kalıyor | Beklenen liste `.feature` dosyalarından sayılır; tag süzgeci (`features.txt`, `ULAK_TAGS`) Maven'a verilenle aynı mı? |
| Koşum yarıda `interrupted` oldu | Test 15 dk boyunca hiç olay/sonuç üretmediyse; uzun hazırlık varsa `RUN_STALL_TIMEOUT_SECS`'i büyüt |
| Sunucu kısa süre kapandı | Kayıp yok: watch tekrar dener, report işindeki toplu zip eksikleri tamamlar, tekrarlar sayılmaz |
