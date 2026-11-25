# Python'a geçiş rehberi

Rust sürümünde aşılamayan bir sorun çıkarsa divan ve ulak Python'a taşınabilir. Bu rehber
taşımayı adım adım anlatır. Her adımın sonunda hangi testin geçmesi gerektiği yazılıdır.

Hazır başlangıç: **[`docs/python-iskelet/`](python-iskelet/)**. Çalışan bir Python iskeleti
içerir: Allure ayrıştırma, veritabanı, defterdar, zip yükleme, CI uçları, canlı durum, WebSocket ve
`ulak watch`. Rust testlerinin 10 tanesinin Python karşılığı da var ve hepsi gerçek örnek
verilerle (`testdata/`) geçiyor. Rust `ulak upload` ve `ulak finish` bu Python sunucusuna
karşı da denendi ve çalıştı (11 sonuç, 7 senaryo, 4 geçti):

```bash
cd docs/python-iskelet
python -m venv .venv && . .venv/bin/activate     # Windows: .venv\Scripts\activate
pip install -r requirements.txt
pytest -q                                         # 10 passed
```

Dosya başına davranış belgeleri `docs/kod/` altında. Taşırken her modülün belgesini açın: bu
rehber **sırayı ve tuzakları**, `docs/kod/*.md` ise **ayrıntılı davranışı** anlatır.

İçindekiler:

1. [Değişmemesi gerekenler (sözleşmeler)](#1-degismemesi-gerekenler-sozlesmeler)
2. [Teknoloji seçimi](#2-teknoloji-secimi)
3. [Proje yapısı](#3-proje-yapisi)
4. [Taşıma sırası](#4-tasima-sirasi)
5. [Kritik parçalar ve tuzaklar](#5-kritik-parcalar-ve-tuzaklar)
6. [Sayfalar, şablonlar ve grafikler](#6-sayfalar-sablonlar-ve-grafikler)
7. [Arka plan işleri](#7-arka-plan-isleri)
8. [ulak (CI aracı)](#8-ulak-ci-araci)
9. [Paketleme ve kurulum](#9-paketleme-ve-kurulum)
10. [Doğrulama: Python sürümü Rust'ın yerine geçebilir mi?](#10-dogrulama)
11. [Bellek ve performans](#11-bellek-ve-performans)
12. [Kontrol listesi](#12-kontrol-listesi)

---

## 1. Değişmemesi gerekenler (sözleşmeler)

Python sürümü bunları **birebir** korumalıdır. Korursa test projeleri, CI şablonu, mevcut
veritabanı ve `.env` dosyası hiç değiştirilmeden çalışmaya devam eder.

| Sözleşme | Nerede tanımlı | Neden sabit |
|----------|----------------|-------------|
| HTTP uçları, yöntemleri, durum kodları (401, 409, 413, 422, 202, 204) | `docs/kod/divan/web/ci.md`, `web/mod.md` | ulak ve CI şablonu bunlara bağlı |
| JSON alan adları (`run_id`, `accepted_results`, `report_url`, `hata`, …) | `docs/kod/common/api.md` | ulak yanıtları bu adlarla okur |
| Canlı olay biçimi (`tur`, `ad`, `uri`, `satir`, `metin`, `durum`, `sure_ms`, `hata`) | `docs/ANLIK_AKTARIM.md` | Java eklentisi bu biçimde yazar |
| WebSocket anlık görüntüsü (`status`, `progress`, `current`, `scenarios[].state` …) | `docs/kod/divan/live.md` | `live.html` betiği bu alanları okur |
| Veritabanı şeması | `migrations/*.sql` | mevcut `divan.db` taşınmadan açılır |
| Dosya düzeni: `DATA_DIR/divan.db`, `DATA_DIR/runs/<id>/files/<allure-adı>`, `DATA_DIR/tools/`, `DATA_DIR/logs/` | `docs/kod/divan/paths.md` | yedekler ve ekler aynı yerde kalır |
| Ortam değişkenleri ve `.env` | `docs/kod/divan/config.md`, `.env.example` | kurulu sunucuların ayarları |
| Admin şifre hash'i (argon2id, PHC biçimi `$argon2id$v=19$…`) | `docs/kod/divan/auth.md` | `argon2-cffi` aynı hash'i doğrular, şifre yeniden belirlenmez |
| ulak alt komutları, ortam değişkenleri (`ULAK_*`), çıkış kodları (0, 1, 2 sürüm, 3 bellek, 4 oturum) | `docs/kod/ulak/*.md` | CI şablonu ve iş sonuçları bunlara bağlı |
| ulak durum dosyaları: `.ulak/run.json`, `sent.txt`, `stop`, `queue/`, `ulak.env` | `docs/kod/ulak/ctx.md` | GitLab artifact'ları ve dotenv |
| `x-ulak-version` uyumu (major.minor) | `docs/kod/divan/auth.md` | eski ulak açık mesajla reddedilir |
| Log satırı biçimi (JSON, `run_id` alanı) | `docs/kod/divan/logging.md` | CI çıktısı ile sunucu logu eşleşir (SW-10.7) |

Sürüm numarasını Rust'tan devam ettirin (ör. `0.2.0`). Böylece eski ulak ile yeni sunucu
birbirini tanır; taşıma sırasında karışık sürüm kalmaz.

## 2. Teknoloji seçimi

| Rust | Python | Not |
|------|--------|-----|
| axum + tokio | **FastAPI** + **uvicorn** | iskelette sınandı (FastAPI 0.162, Starlette 1.10) |
| axum WebSocket | FastAPI `WebSocket` | iskelette `/ws/runs/{id}` |
| askama | **Jinja2** (`autoescape=True`) | şablonlar küçük değişiklikle aynen kullanılır (§6) |
| rusqlite (bundled) | `sqlite3` (standart kütüphane) | SQLite ≥ 3.25 gerekir (pencere fonksiyonları); kontrol: `python -c "import sqlite3; print(sqlite3.sqlite_version)"` |
| argon2 | `argon2-cffi` | |
| rust-embed | paket verisi (`importlib.resources`) ya da kurulum klasörü | |
| ureq (ulak) | `requests` | `session.trust_env = False` |
| clap | `argparse` | |
| zip | `zipfile` | dosyadan akış halinde okur, belleğe almaz |
| lettre | `smtplib` | düz relay, TLS yok (iç ağ) |
| tracing | `logging` + JSON biçimleyici | |

**Python sürümü: 3.12 ya da 3.13.** Bağımlılık sürümleri `docs/python-iskelet/requirements.txt`
dosyasında sabitlendi. Kapalı ağda kurmak için bir internet makinesinde
`pip download -r requirements.txt -d paketler/` (hedef işletim sistemi için ayrıca
`--platform win_amd64 --only-binary=:all:`) çalıştırın, sonra hedefte
`pip install --no-index --find-links paketler/ -r requirements.txt`.

## 3. Proje yapısı

Rust crate'leri Python paketlerine bire bir eşlenir; modül adları da aynı kalır. Böylece
`docs/kod/` belgeleri doğrudan yol gösterir.

```
divan-py/
├── pyproject.toml
├── migrations/              → mevcut klasör (kopyalanmaz, yolu verilir ya da pakete eklenir)
├── divan/                 → crates/divan/src
│   ├── __main__.py          main.rs   (serve | set-admin-password | backup)
│   ├── app.py               lib.rs    (create_app, router, arka plan işleri)
│   ├── config.py  paths.py  db.py  logging_json.py  auth.py
│   ├── allure.py            crates/common/src/allure.rs  (ulak ile paylaşılır)
│   ├── api.py               crates/common/src/api.rs     (pydantic modelleri)
│   ├── defterdar.py  upload.py  live.py  hooks.py  stall.py  disk.py
│   ├── queries.py  history.py  charts.py  export.py  share.py
│   ├── triage.py  fix.py  jira.py  gitlab.py  email.py
│   ├── web/                 ci.py  pages.py  admin.py  files.py  gitlab_hook.py  tools.py
│   ├── templates/           → crates/divan/templates (kopya, §6)
│   └── static/              → crates/divan/static (aynen)
├── ulak/                  → crates/ulak/src
│   ├── __main__.py  ctx.py  client.py  gherkin.py  preflight.py
│   └── runner.py  watch.py  upload.py  fix.py
└── tests/                   → crates/divan/tests (pytest)
```

`allure.py` hem sunucuda hem ulak'ta kullanılır. ulak ayrı ikili olarak paketlenirken
(PyInstaller) aynı modül içine alınır; ayrı bir "common" paketi gerekmez.

## 4. Taşıma sırası

Her adım kendi başına çalışan bir sürüm bırakır. Bir adım bitmeden sonrakine geçmeyin. "Rust
belgesi" sütunu o adımda okunacak `docs/kod/` dosyalarıdır.

| # | Adım | Rust belgesi | Bitti sayılması için |
|---|------|--------------|----------------------|
| 1 | Yapılandırma, yollar, JSON log, `/api/health`, statik dosyalar | `config`, `paths`, `logging`, `web/mod` | `temel.rs`: sağlık 200, statik dosya |
| 2 | Veritabanı + migration'lar; mevcut bir `divan.db`'yi açabilmek | `db`, `diger/migrations` | Rust'ın oluşturduğu veritabanı hatasız açılır, `schema_migrations` iki satır |
| 3 | CI koruması (token, sürüm) | `auth` | `temel.rs`: 401, 409 |
| 4 | Allure ayrıştırma + defterdar + zip yükleme (`POST /api/ci/runs`) | `common/allure`, `defterdar`, `upload`, `web/ci` | `sonuc_alma.rs` tamamı: 11 sonuç, 7 senaryo, 4/1/1/1; `../` 400; sonuçsuz 422; tekrar sayılmaz; rerun |
| 5 | Koşum aç / tek dosya / bitir (`/runs/start`, `/results`, `/finish`, `/upload`) | `web/ci`, `hooks`, `stall` | `temel.rs`: durgun koşum `interrupted`; `cargo run -p e2esmoke` (ULAK_SERVER ile) ilk yarısı |
| 6 | Canlı durum: olaylar, anlık görüntü, WebSocket | `live`, `ANLIK_AKTARIM.md` | `rapor.rs`: `test_sw_3_4_*` üçü |
| 7 | Sayfalar: dashboard, koşum, senaryo, feature, geçmiş, arama, CSV | `web/pages`, `queries`, `history`, `charts`, `export`, `templates/README` | `rapor.rs` geri kalanı; sayfalar Rust sürümüyle yan yana karşılaştırılır |
| 8 | Admin girişi, paylaşım linkleri, ek dosyası sunumu | `auth`, `web/admin`, `share`, `web/files` | `temel.rs` SW-9.5, `rapor.rs` SW-5.1/5.2 |
| 9 | Arka plan: durgunluk, disk temizliği, yedek, log temizliği | `stall`, `disk`, `db`, `logging` | `rapor.rs` SW-10.8 |
| 10 | Triage, Jira, GitLab webhook, e-posta, self-healing kayıtları | `triage`, `jira`, `gitlab`, `web/gitlab_hook`, `email`, `fix` | `entegrasyon.rs` tamamı |
| 11 | ulak: `preflight`, `start`, `watch`, `upload`, `finish` | `ulak/*` | `cargo run -p rehearse` (§10) fix adımları hariç |
| 12 | ulak `fix` | `ulak/fix`, `KOD_AJANI_VLLM.md` | `rehearse` tamamı (44/44) |
| 13 | Paketleme: Linux ve Windows | §9 | `e2esmoke` 18/18 (§10) |

İskelet 4., 5. ve 6. adımların çekirdeğini ve 11. adımdaki `watch`'u içerir. Bunlar taşımanın en
riskli kısımlarıdır; geri kalan adımlar çoğunlukla SQL sorgusu ve şablon işidir.

**Geçiş sırasında iki sürüm yan yana çalışabilir:** Python sunucusunu başka bir portta ve
`DATA_DIR`'in **kopyasıyla** açın, aynı zip'i ikisine de yükleyin ve sayfaları karşılaştırın.
Aynı `divan.db`'yi iki sunucu aynı anda yazmamalıdır.

## 5. Kritik parçalar ve tuzaklar

### 5.1 Allure ayrıştırma — `python-iskelet/divan/allure.py`

- `status` alanı **hiç olmayabilir** (tanımsız adımlı senaryo); `unknown` sayın, hata vermeyin.
- Senaryo kimliği `historyId` (yoksa `testCaseId` → `fullName` → `name`). Rerun'da aynı kalır.
- `@After` hook'unda eklenen ekran görüntüsü ve video **result dosyasında değil container
  dosyasındaki `afters[].attachments`** altındadır.
- Cucumber ek dosyalarının uzantısı yoktur (`...-attachment`); türü `type` alanındadır.
- Tüm alanları `.get()` ile okuyun; bilinmeyen alanlar yok sayılır.

### 5.2 Defterdar ve `is_latest` — `python-iskelet/divan/defterdar.py`

- `scenarios` tablosunda her satır **bir denemedir**. Koşumdaki sonuç, aynı `scenario_key`'in son
  denemesidir (`is_latest=1`). Her yüklemeden sonra `recompute_run` çağrılır ve sayaçlar
  yeniden hesaplanır. **Tüm rapor sorguları `is_latest = 1` ile süzmelidir.**
- Aynı `result_uuid` ikinci kez gelirse hiçbir şey yapılmaz (ulak aynı dosyayı yeniden
  gönderebilir).
- Container dosyası sonuçtan **önce de sonra da** gelebilir; `link_fixture_attachments` her
  yüklemede çağrılır ve bağlanmamış ekleri bağlar.
- `tags`, `doors_ids`, `jira_keys` boşlukla çevrili metin olarak saklanır (`" giris smoke "`).
  Arama `LIKE '% giris %'` ile yapılır; bu biçimi bozmayın.

### 5.3 Zip yükleme — `python-iskelet/divan/upload.py`

- Gövdeyi **asla** `await request.body()` ile belleğe almayın; `request.stream()` ile diske
  yazın (`body_to_file`). Sınır aşılırsa 413.
- İki geçiş: önce içeriği açmadan tüm adları denetleyin (`../`, mutlak yol, `:`), tek bir kötü ad
  **bütün yüklemeyi** 400 ile reddeder. Sonra işleyin.
- Zip işleme bloklayıcıdır; `asyncio.to_thread` ile ayrı iş parçacığında çalıştırın. Yoksa o
  sırada canlı ekran ve diğer istekler donar.
- Ekleri önce `.ad.part` olarak yazıp `os.replace` ile yeniden adlandırın; yarım dosya görünmez.
- Hata olursa transaction geri alınır ve yeni koşumun klasörü silinir.

### 5.4 Veritabanı eşzamanlılığı — `python-iskelet/divan/db.py`

- SQLite'ta aynı anda tek yazıcı olur. Tek yazma bağlantısı ve `threading.Lock` kullanın; okumalar
  iş parçacığı başına ayrı bağlantıdan yapılır (WAL kipinde okuyucular yazıcıyı beklemez).
- `isolation_level=None` verip transaction'ları elle açın (`BEGIN IMMEDIATE` … `COMMIT`).
  Python'un örtük transaction davranışı `executescript` ve `WITH … UPDATE` ile karışıklık çıkarır.
- **uvicorn'u tek işçiyle çalıştırın (`--workers 1`).** Canlı durum ve WebSocket yayını süreç
  belleğindedir; birden fazla işçi olursa olaylar bir işçiye, WebSocket başka işçiye düşer ve canlı
  ekran güncellenmez.

### 5.5 Canlı durum — `python-iskelet/divan/live.py`, `app.py` → `snapshot`

- Sonuç önceliği: **Allure sonucu > `senaryo_bitti` olayı > şu an koşan > bekliyor**.
- Rerun'da aynı `uri|satır` iki kez başlar. Son durum geçerlidir, `attempt` artar.
- Senaryo taslağının satırları **aynı adı** taşır. Eşleştirirken her sonucu ve olayı bir kez
  tüketin (`pop(0)`). "Koşuyor" yalnızca ilk eşleşen satıra verilir. İskelette bu hata testle
  yakalandı ve düzeltildi.
- Bitmiş koşuma gelen olaylar 202 ile yok sayılır. Koşum bitince bellek durumu silinir.
- WebSocket'te 250 ms içinde gelen sinyalleri birleştirip tek görüntü gönderin; her olayda ayrı
  gönderim yapılırsa tarayıcı boğulur.
- İskeletteki `snapshot` basitleştirilmiştir. Kalan senaryonun ekran görüntüsü
  (`screenshot_id`), deneme numarası ve beklenmeyen sonuçların listeye eklenmesi Rust
  `live.rs`'ten tamamlanmalıdır.

### 5.6 Gizli değerler

Tokenları tutan değişkenler için `__repr__`'i `"***"` döndüren küçük bir sınıf kullanın. Jira,
GitLab ve SMTP'den gelen hata mesajlarını loga ya da kullanıcıya vermeden önce `redact()` ile
temizleyin (`config.md`). `entegrasyon.rs`'teki `test_sw_9_2_*` bunu denetler; Python'a da taşıyın.

## 6. Sayfalar, şablonlar ve grafikler

- `crates/divan/templates/*.html` dosyalarını kopyalayın. Askama'ya özgü birkaç yapı değişir
  (tam liste: `docs/kod/divan/templates/README.md`):

  | Askama | Jinja2 |
  |--------|--------|
  | `{% if let Some(x) = alan %}` | `{% if alan is not none %}{% set x = alan %}` |
  | `x.as_deref()`, `x.is_some()`, `x.len()` | `x`, `x is not none`, `x\|length` |
  | `{% match %}` | `if / elif` |

  Bu değişiklikler `sed` ile değil elle yapılmalı; `grep -n "if let\|as_deref\|is_some\|is_none\|\.len()\|match" templates/*.html` hepsini listeler.
- `Environment(autoescape=True)`. HTML üreten yardımcılar (SVG grafikler) `Markup(...)` ile
  işaretlenir; şablonda `|safe` aynı kalır.
- Ortak `l` (Layout) nesnesi: `admin`, `share`, `banner` ve `run_url(id)`, `scenario_url(id)`,
  `file_url(id)` metotları. Paylaşım görünümünde linkler `/s/<token>/…` olur (`pages.md`).
- **Grafikler:** `charts.rs` saf fonksiyonlardan oluşur (sayı → SVG metni). Python'a satır satır
  çevrilir: `donut`, `thin_bar`, `line_pct`, `stacked_columns(_sized)`, `sparkline`, `nice_max`
  (adımlar {1,2,3,4,5,6,8}×10ⁿ), `pct` (yuvarlama, lejant ile donut ortası aynı olmalı).
  `charts.rs`'teki birim testlerini de taşıyın; en kolay doğrulama, aynı girdi için **aynı SVG
  metninin** çıkmasıdır.
- `static/` aynen kopyalanır; `Cache-Control: public, max-age=3600`.

## 7. Arka plan işleri

FastAPI `lifespan` içinde `asyncio.create_task` ile başlatın. Bloklayıcı işleri (SQL, dosya
silme) `asyncio.to_thread` ile çalıştırın.

| İş | Aralık | Rust belgesi |
|----|--------|--------------|
| Durgun koşumu `interrupted` yapma | `RUN_STALL_TIMEOUT_SECS / 10` (1–30 sn) | `stall.md` |
| Disk ölçümü ve temizlik (önce videolar, sonra görüntüler, en eski koşumdan) | 10 dk | `disk.md` (`shutil.disk_usage`) |
| Gecelik yedek (`VACUUM INTO`), `BACKUP_KEEP` kadar tutma | 5 dk'da bir saat kontrolü | `db.md` |
| Log temizliği (gün ve boyut sınırı) | 1 saat | `logging.md` |

## 8. ulak (CI aracı)

ulak CI'da çalışır ve sunucudan bağımsızdır. Bu yüzden **ilk taşınabilecek parça budur**: Rust
sunucusuyla Python ulak (ya da tersi) birlikte çalışabilir. Sözleşmeler §1'deki gibi kalmalıdır.

- `__main__.py`: `argparse` alt komutları; `sys.exit(kod)`. Beklenmeyen hata
  `[ulak] HATA: …` + çıkış 1.
- `ctx.say()`: her satır `[ulak run_id=12] …` biçiminde (SW-10.7).
- `client.py`: `requests.Session`, `trust_env=False`, `timeout=(5, 600)`, `Authorization` ve
  `x-ulak-version` başlıkları; ağ hatası ve 5xx'te artan beklemeyle yeniden deneme
  (`ULAK_RETRY_MAX`, `ULAK_RETRY_BASE_MS`), 4xx'te hemen dönüş.
- `runner.py` (`start`): `subprocess.Popen(..., stdout=PIPE, stderr=STDOUT, encoding="utf-8",
  errors="replace")` ile satır satır okuyup basın ve OOM satırlarını yakalayın. Komutu **kabuksuz**
  çalıştırın (`shell=False`); Windows'ta `mvn` yerine `mvn.cmd`. İzleyiciyi kendi ikilisiyle
  başlatın: PyInstaller ikilisinde `sys.executable` ikilinin kendisidir →
  `[sys.executable, "watch", "--run-id", id]`; Python ile çalışırken
  `[sys.executable, "-m", "ulak", "watch", …]`.
- `watch.py`: iskelette hazır ve gerçek bir uvicorn sunucusuna karşı sınandı. Yalnızca tam
  satırlar gönderilir; konum yalnızca başarıda ilerler; ekler json'lardan önce gönderilir; yazımı
  süren dosya atlanır.
- `gherkin.py`: satır tabanlı ayrıştırıcı ve tag ifadesi (`or < and < not`), `gherkin.md`. Türkçe
  anahtar kelimeler (`Özellik`, `Senaryo taslağı`, `Örnekler` …) unutulmamalı. Test: örnek proje
  7 senaryo verir.
- `upload.py`: `zipfile` ile; `.json`/`.txt` deflate, görüntü/video stored. Ağ hatasında zip
  `.ulak/queue/`'ya yazılır; `upload --resume` sonra gönderir.
- `fix.py`: `subprocess.run(["git", …])`, ajan zaman aşımı `timeout=`. Push URL'si token içerir;
  hata mesajlarından tokenı silin. Yol denetimi (`src/test/` dışı değişiklik → hepsi geri alınır)
  ve assertion uyarısı ajandan bağımsız olarak ulak'ta kalmalıdır.
- Windows hizmet oturumu kontrolü: kullanıcı adı `$` ile bitiyorsa ya da `SESSIONNAME` yoksa veya
  `Services` ise çıkış 4 (`runner.md`).

## 9. Paketleme ve kurulum

### ulak → tek dosya

```bash
pip install pyinstaller
pyinstaller --onefile --name ulak ulak/__main__.py     # Linux'ta: dist/ulak
```

Windows ikilisi **Windows'ta** derlenir (PyInstaller çapraz derleme yapmaz):
`pyinstaller --onefile --name ulak ulak\__main__.py` → `dist\ulak.exe`. Çıktılar sunucuda
`DATA_DIR/tools/ulak-linux` ve `DATA_DIR/tools/ulak-windows.exe` olarak durur; CI şablonu
değişmez. Boyut Rust'tan büyük olur (Rust ulak ~2,4 MB; PyInstaller ile birkaç kat), iç ağda indirme süresine etkisi önemsizdir.

### divan sunucusu

**Linux (systemd):** `deploy/linux/divan.service` içinde yalnızca `ExecStart` değişir:

```ini
ExecStart=/opt/divan/.venv/bin/python -m divan serve
```

`serve` komutu uvicorn'u programdan başlatmalıdır:
`uvicorn.run(app, host=…, port=…, workers=1)`. `MemoryMax=150M` kalabilir (§11).

**Windows:** İki yol var:
1. Sunucu makinesine Python kurmak istemiyorsanız PyInstaller ile `divan.exe`. Şablonları,
   `static/` ve `migrations/` klasörlerini `--add-data` ile içine koyun ve kodda
   `sys._MEIPASS`'tan okuyun.
2. Python kuruluysa venv + `python -m divan serve`.

İki yolda da `deploy/windows/kurulum.ps1` içindeki zamanlanmış görevin komut satırı değişir;
gerisi aynı kalır.

`set-admin-password` ve `backup` alt komutları da taşınmalıdır (`main.md`): `ULAK_NEW_ADMIN_PASSWORD`
ve `ENV_FILE` ortam değişkenlerini okur, `.env`'de yalnızca `ADMIN_PASSWORD_HASH='…'` satırını
değiştirir.

## 10. Doğrulama

Python sürümünün Rust'ın yerine geçebildiğini üç katmanda gösterin:

1. **Birim/entegrasyon (pytest):** `crates/divan/tests/*.rs` testleri `pytest`'e çevrilir.
   Girdiler (`testdata/` zip'leri, istekler) ve beklentiler (durum kodu, sayılar, sayfadaki metin)
   aynıdır. Örnekler: `docs/python-iskelet/tests/test_iskelet.py`. Test adları aynı kalsın
   (`test_sw_2_1_…`); böylece gereksinim izlenebilirliği korunur.

2. **Smoke (kurulu sunucuya karşı):** `e2esmoke` dışarıdan HTTP ile sınar; Python sunucusunu
   açıp şunu çalıştırın:

   ```bash
   ULAK_SERVER=http://127.0.0.1:8080 CI_API_TOKEN=<token> cargo run -p e2esmoke
   ```

   Bu kipte smoke yalnızca HTTP üzerinden yapılabilen kontrolleri koşar. Yükleme için Rust
   `ulak`'yi kullanır; bu da sunucunun Rust istemciyle uyumunu gösterir. Admin şifresi, log
   dosyası ve yedek kontrolleri smoke kendi sunucusunu açtığında koşar. Bunlar için Python
   ikilisini aşağıdaki gibi `target/debug/divan` yerine koyup `ULAK_SKIP_BUILD=1
   ./target/debug/e2esmoke` çalıştırın (18 kontrol).

3. **Prova (CI akışı uçtan uca):** `rehearse` ikilileri kendi klasöründen (`target/debug/`)
   çalıştırır. Python ikililerini oraya koyup derlemeyi atlatın:

   ```bash
   cargo build -p rehearse
   cp dist/ulak target/debug/ulak
   # divan için: aynı ortam değişkenlerini (DATA_DIR, BIND_ADDR, PUBLIC_URL, CI_API_TOKEN,
   # DRY_RUN, LOG_STDOUT) okuyup sunucuyu açan bir PyInstaller ikilisi ya da betik
   cp dist/divan target/debug/divan
   ULAK_SKIP_BUILD=1 ./target/debug/rehearse        # 44 kontrolün hepsi geçmeli
   ```

   Rust ulak + Python divan, Python ulak + Rust divan ve ikisi Python olmak üzere üç
   birleşimi de deneyin. Üçü de geçiyorsa taşıma tamamdır.

Gerçek Java projesiyle son kontrol: `ornek-test-projesi` üzerinde `ulak start` çalıştırın,
canlı ekranda şu an koşan senaryo ve adımın göründüğünü, raporda ekran görüntüsü, video ve sahne
ağacının bağlandığını görün (`docs/ANLIK_AKTARIM.md`, "Doğrulama").

## 11. Bellek ve performans

İskelet üzerinde ölçüldü (Python 3.13, FastAPI 0.162, Linux):

| Durum | RSS |
|-------|-----|
| Sunucu boşta | 53 MB |
| 150 MB zip yüklenirken (35 video eki) tepe | 66 MB |

Hedef < 100 MB (SW-10.6) karşılanıyor, ancak Rust'taki ~30 MB'a göre pay daralıyor. Şunlara
dikkat edin:

- Yükleme, CSV dışa aktarma ve dosya sunumu **akış halinde** olmalıdır (`StreamingResponse`,
  `FileResponse`). Hiçbir yerde bütün sonuç listesini belleğe toplamayın; sayfalı sorgu kullanın.
- `SQLITE_CACHE_KIB` (8 MB) ve okuma bağlantısı sayısı belleği doğrudan etkiler.
- Tek uvicorn işçisi (§5.4); ek işçi her biri ~50 MB ekler.
- `tests/bellek.rs`'i Python'a taşıyın: `resource.getrusage(...).ru_maxrss` ya da `/proc/self/status`.

Hız: Python bu yükte (gecede birkaç koşum, saniyede birkaç olay) darboğaz olmaz. En ağır iş zip
işlemedir; ayrı iş parçacığında yapıldığı sürece canlı ekranı etkilemez.

## 12. Kontrol listesi

- [ ] §1'deki sözleşmelerin hepsi korunuyor (uçlar, JSON adları, olay biçimi, şema, `.env`, çıkış kodları)
- [ ] Rust'ın oluşturduğu mevcut `divan.db` ve `DATA_DIR` Python sürümüyle açılıyor
- [ ] Mevcut admin şifresi (argon2 hash) Python'da doğrulanıyor
- [ ] Tüm sorgular `is_latest = 1` ile süzülüyor
- [ ] Zip ve CSV akış halinde; bellek < 100 MB (`bellek` testi)
- [ ] uvicorn tek işçi
- [ ] Tokenlar log ve hata mesajlarında `***`
- [ ] pytest: `crates/divan/tests` karşılıklarının hepsi geçiyor
- [ ] `e2esmoke` kurulu Python sunucusuna karşı geçiyor; ikili değiştirilerek 18/18
- [ ] `rehearse` 44/44 (üç birleşim)
- [ ] ulak Linux ve Windows ikilileri `DATA_DIR/tools/` altında
- [ ] Gerçek JavaFX projesiyle Linux (Xvfb) ve Windows (masaüstü oturumu) koşumu
- [ ] `GEREKSINIMLER_1.md` durumları ve `CLAUDE.md` komutları Python'a göre güncellendi
