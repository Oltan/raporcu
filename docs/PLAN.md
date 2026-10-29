# stajproje Uygulama Planı

Kaynak: `GEREKSINIMLER_1.md` (58 SW gereksinimi). Bu dosya o gereksinimlerin hangi sırayla, hangi
modüllerle ve hangi testle kapanacağını gösterir. Durumlar `GEREKSINIMLER_1.md` içinde tutulur.

## Mimari özet

```
GitLab pipeline (test projesi)                       stajproje makinesi (aynı makine)
┌──────────────────────────────────────┐            ┌─────────────────────────────────────┐
│ test job (linux: xvfb-run / windows: │            │ stajproje (axum + tokio)            │
│ masaüstü oturumu)                    │  localhost │  /api/ci/runs/start      (SW-3.1)   │
│  mvn test → target/allure-results ───┼──stajci───▶│  /api/ci/runs/{id}/results (3.3)    │
│  stajci watch  (paralel)             │   watch    │  /api/ci/runs            (SW-2.1)   │
│ report job: stajci upload / finish   │──upload───▶│  /api/ci/runs/{id}/finish (3.5)     │
│ fix job:    stajci fix (OpenCode)    │            │  SQLite (WAL) + DATA_DIR dosyaları  │
└──────────────────────────────────────┘            │  askama + htmx sayfalar, WebSocket  │
                                                    └─────────────────────────────────────┘
```

Cargo workspace:

| crate        | görev                                                                  |
|--------------|------------------------------------------------------------------------|
| `common`     | Allure modeli + ayrıştırıcı, ortak API tipleri, sürüm bilgisi          |
| `stajproje`  | Sunucu: yapılandırma, `paths.rs`, veritabanı, HTTP uçları, sayfalar    |
| `stajci`     | CI yardımcı ikilisi: `preflight`, `start`, `watch`, `upload`, `finish`, `fix` |
| `rehearse`   | Prova: CI akışını yerelde benzetir (SW-10.2)                           |
| `e2esmoke`   | Uçtan uca smoke (SW-10.2)                                              |

Yardımcı klasörler:

- `migrations/` — numaralı SQL dosyaları, ikiliye gömülür (SW-2.5)
- `testdata/allure/` — gerçek `allure-results` örnekleri (SW-2.7)
- `ornek-test-projesi/` — JavaFX + Cucumber + TestFX + Allure örnek test projesi. Gerçek örnekleri
  üretir ve kendi test projene Allure eklerken referans olur.
- `ci/gitlab/template/` — merkezi CI şablonu (SW-1.1, 1.4)

## Fazlar

Her faz bir öncekinin üzerine kurulur. Bir seferde tek SW; her SW kendi testiyle
(`test_sw_x_y_...`) kapanır ve durumu `MEVCUT` yapılır.

| Faz | Gereksinimler | Çıktı |
|-----|---------------|-------|
| 1 Temel | 10.3 (M), 10.5, 2.4, 2.5, 10.7, 10.6, 9.1, 9.5 | Workspace, sağlık ucu, DATA_DIR, SQLite + migration + yedek, JSON log, bellek sınırları, CI token, admin girişi |
| 2 Sonuç alma | 2.7, 4.6, 2.1, 2.3, 2.2 | Gerçek Allure örnekleri, proje/branch, zip yükleme, 422, @After ekleri |
| 3 CI hattı | 1.1, 1.2, 1.6, 1.7, 1.5, 10.9, 2.6, 10.2, 1.3 | `.gitlab-ci.yml` şablonu, Xvfb/Windows oturumu, bellek, ön kontrol, stajci dağıtımı, kuyruk, prova/smoke |
| 4 Raporlama | 4.1, 4.3, 4.9, 4.2, 4.7, 4.8, 10.8 | Rapor sayfaları, CI bilgisi, rerun, flaky, arama, CSV, disk temizliği |
| 5 Paylaşım | 5.1, 5.4, 5.2 | Salt okunur link, pipeline linki, iptal |
| 6 Canlı durum | 3.1, 3.3, 3.2, 3.4, 3.5, 9.3, 3.6 | start/results/finish uçları, `stajci watch`, WebSocket ekran, webhook |
| 7 Jira ve DOORS | 6.1, 6.4, 6.2, 6.3, 5.3 | Jira istemcisi, DOORS CSV/DXL, eşleşme, gözlem modu, e-posta |
| 8 Triage ve self-healing | 4.5, 7.1, 7.6, 7.2, 9.4, 7.3, 4.4, 9.2, 7.4, 7.5, 7.7 | Triage, OpenCode fix akışı ve koruması, ölçüm paneli |
| 9 Yaygınlaştırma | 1.4, 10.1, 10.4 | Merkezi include, kurulum ve yeni proje rehberleri |

## Allure veri modeli (ayrıştırıcının temeli)

`allure-results/` klasöründe allure-cucumber7-jvm şunları üretir:

- `*-result.json` — bir senaryo denemesi: `uuid`, `historyId`, `testCaseId`, `name`, `fullName`,
  `status` (`passed|failed|broken|skipped|unknown`), `statusDetails.message/trace`, `stage`,
  `start/stop`, `labels` (feature, tag, suite, host, thread...), `steps[]`, `attachments[]`, `parameters[]`.
- `*-container.json` — `children[]` (result uuid'leri), `befores[]`, `afters[]`. Cucumber
  `@Before/@After` hook'ları buradadır; `@After` içinde alınan ekran görüntüsü `afters[].attachments`
  altındadır. SW-2.2 bu ekleri `children` üzerinden senaryoya bağlar.
- `*-attachment.png|txt|...` — ek dosyaları; json'daki `source` alanı dosya adını verir.

Eşleştirme kuralları:

- Senaryo kimliği: proje + `historyId` (yoksa `fullName`). Aynı koşumda aynı `historyId` birden çok
  kez gelirse en son `stop` zamanlı deneme sonuçtur, öncekiler deneme listesidir (SW-4.9).
- Tekrar sayılmama: sonuç dosyasının `uuid` değeri koşum içinde benzersizdir (SW-3.3).
- DOORS / Jira: `@DOORS-xxx` ve `@JIRA-xxx` biçimli tag'ler (`labels[name=tag]`) ayrı alanlara çıkarılır.

## Açık sorular (başlamadan önce sorulacak)

1. GitLab token yetkisi (SW-7.4, 7.5) — faz 8'den önce.
2. Ubuntu ve Windows runner'ları ayrı makinedeyse tek merkezi sunucu mu? (SW-3.2, 3.6, 4.2) — faz 6'dan önce.
3. Triage ayrıntıları (SW-4.5) — faz 8'den önce.

## Durum

Tüm fazlar uygulandı: 58 gereksinimden 34'ü MEVCUT, 23'ü KISMEN, 1'i YAPILACAK (SW-10.3,
şirket GitLab'ı gerekiyor). KISMEN olanların hepsinde kod ve buradaki testler tamam; eksik kısım
GitLab pilotu, Windows VM ya da gerçek Jira / OpenCode / SMTP ile doğrulamadır:
`docs/PILOT_KONTROL_LISTESI.md`. Açık sorulardaki varsayımlar: `docs/KARARLAR.md`.

| Doğrulama | Sonuç (Linux) |
|-----------|---------------|
| `cargo clippy --all-targets -- -D warnings` | temiz |
| `cargo test` | 95 test geçiyor |
| `cargo run -p rehearse` | 43/43 adım |
| `cargo run -p e2esmoke` | 18/18 adım |
| `cargo build --release` (Linux, `x86_64-pc-windows-gnu`) | derleniyor |
| Gerçek JavaFX/TestFX koşusu Xvfb + `stajci start/upload/finish` | geçti |
| Release sunucu bellek (200 MB zip + 40 eşzamanlı istek) | tepe ~28 MB |
