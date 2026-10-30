# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Ne bu proje

divan: GitLab CI'da koşan JavaFX (Cucumber + TestFX) e2e testlerinin Allure sonuçlarını alan
test sonuç merkezi. İki ikili: **`divan`** (sunucu: axum + askama/htmx + WebSocket + SQLite) ve
**`ulak`** (CI aracı; CI şablonu yalnızca onun alt komutlarını çağırır). Sunucu ve GitLab runner
aynı makinede (Ubuntu ya da Windows), iç ağ, düz HTTP. Gereksinimler `GEREKSINIMLER_1.md`
(SW-x.y numaralı). Arayüz, belgeler, log mesajları ve kod içi tanımlayıcıların çoğu **Türkçe**.

Eski adlar: `raporcu` → `divan`, `stajci` → `ulak`, `STAJ_*` → `ULAK_*`, `ingest` modülü →
`defterdar` (`docs/KARARLAR.md`). GitHub deposu ve yerel klasör şimdilik `raporcu` adındadır.

## Komutlar

```bash
cargo build                                   # tüm workspace
cargo clippy --all-targets -- -D warnings     # uyarı = hata (clippy.toml: type-complexity-threshold=600)
cargo fmt
cargo test                                    # birim + crates/divan/tests entegrasyon testleri
cargo test -p divan --test rapor test_sw_4_1    # tek dosya / tek gereksinim
cargo test sw_3_4                             # bir gereksinimin tüm testleri (ad kuralı)
cargo run -p rehearse                         # CI provası: gerçek divan + ulak süreçleri, sahte mvn/ajan/GitLab
cargo run -p e2esmoke                         # uçtan uca smoke (ULAK_SERVER + CI_API_TOKEN verilirse kurulu sunucuya)
```

Windows hedefi (Linux'ta mingw + Wine; `.cargo/config.toml` `runner = "wine"`):

```bash
cargo test --target x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu -p divan -p ulak -p rehearse -p e2esmoke
cd target/x86_64-pc-windows-gnu/debug && ULAK_SKIP_BUILD=1 wine rehearse.exe
```

Gerçek Java örnek projesi (JDK 21, Maven, Xvfb gerekir):

```bash
cd ornek-test-projesi && xvfb-run -a mvn -B test
```

Yerel sunucu: `cp .env.example .env`, `cargo run -p divan -- set-admin-password`, `cargo run -p divan`.

## Mimari (birkaç dosyayı okumadan görünmeyenler)

**CI akışı** (`ci/gitlab/template/.gitlab-ci.yml`, aşamalar test → report → fix):
`ulak preflight` → `ulak start` (beklenen senaryoları `.feature` dosyalarından kendisi sayar,
`POST /api/ci/runs/start`, `ulak watch`'u **ayrı süreç** olarak başlatır, `mvn test`'i Xvfb içinde
koşar) → `ulak upload` (toplu zip; sunucu yoksa `.ulak/queue/`'ya, `upload --resume`) →
`ulak finish` → `ulak fix` (OpenCode/Claude Code ile `auto-fix/...` branch + MR). Çıkış kodları
anlamlıdır: 1 test/yükleme hatası, 2 sürüm uyumsuz, 3 bellek (OOM), 4 Windows hizmet oturumu.

**İki canlı kanal** (`docs/ANLIK_AKTARIM.md`): (1) Java tarafındaki `CanliOlayEklentisi`
Cucumber olaylarını JSON-lines dosyasına yazar → `watch` tam satırları
`POST /api/ci/runs/{id}/events`'e gönderir → bellekteki `live::LiveStates` ("şu an koşan senaryo/
adım"); (2) Allure dosyaları senaryo bitince → `POST /api/ci/runs/{id}/results?name=` → veritabanı.
`live::snapshot` öncelik sırası: Allure sonucu > `senaryo_bitti` olayı > aktif senaryo > bekliyor.
WebSocket `/ws/runs/{id}` aynı anlık görüntüyü yayınlar.

**Defterdar modeli** (`crates/divan/src/defterdar.rs`, `migrations/`): `scenarios` tablosunda her satır
bir *deneme*dir (Allure result). Aynı `scenario_key` (= Allure `historyId`) aynı koşumda tekrar
gelirse `attempt+1`, yalnızca son deneme `is_latest=1`. Koşum sayıları her yüklemeden sonra
`is_latest=1` satırlarından yeniden hesaplanır; tüm sorgular `is_latest=1` ile süzmeli. Aynı
`result_uuid` ikinci kez sayılmaz. `@Before/@After` ekleri container dosyaları üzerinden senaryoya
bağlanır (container sonuçtan önce de sonra da gelebilir).

**Sunucu yapısı:** `lib.rs` `AppState` + router; `web/ci.rs` token korumalı CI uçları
(`x-ulak-version` uyumu `common::api`); `web/pages.rs` askama sayfaları; grafikler sunucuda SVG
(`charts.rs`), JS yalnızca ipucu için. Veritabanı: tek yazıcı bağlantı + okuyucu havuzu, WAL;
migration'lar ikiliye gömülü ve `db.rs`'deki listeye eklenmeli. Statik dosya ve şablonlar ikiliye
gömülü (rust-embed `debug-embed`). Paylaşım linkleri aynı şablonları `Layout.share` ile salt okunur
ve iç bilgiler gizli render eder. Koşum sonrası işler (`hooks.rs`): triage kuralları, Jira (varsayılan
gözlem modu), e-posta, canlı durumu temizleme.

**Bellek hedefi** sunucu < 100 MB (SW-10.6): zip'ler akış halinde işlenir, CSV akış halinde yazılır;
`tests/bellek.rs` bunu ölçer.

## Kurallar

- Test adları `test_sw_<bölüm>_<madde>_<açıklama>`; yeni davranış ilgili gereksinim numarasıyla
  test edilir. Yeni CI adımı `crates/rehearse/src/main.rs`'e bir `ck.check("SW-x.y", …)` olarak eklenir.
- Bir gereksinim değişince `GEREKSINIMLER_1.md` içindeki durumunu (`YAPILACAK`/`KISMEN`/`MEVCUT`) ve
  özet sayıları güncelle; CI gereksinimi iki işletim sisteminde doğrulanmadan MEVCUT sayılmaz.
  Açık sorulara verilen varsayımlar `docs/KARARLAR.md`.
- Testler gerçek Allure çıktılarını kullanır: `testdata/allure/ornek-giris/`
  (allure-cucumber7-jvm 2.35.5) ve `testdata/canli/` (gerçek Cucumber olay kaydı). Elle uydurulmuş
  Allure JSON yerine bunları kullan/genişlet.
- Tokenlar (`CI_API_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, `ULAK_FIX_TOKEN`) yalnızca `.env`/CI
  değişkeninde; log ve hata mesajlarına yazılmaz (testlerle denetleniyor).
- Self-healing yalnızca test projesinin `src/test/` yolunu değiştirebilir (`ulak fix` yol denetimi);
  istemde test çıktısı `<<<VERI-…>>>` blokları içinde veridir.
- Monocle/headless JavaFX kullanılmaz (preflight reddeder); Linux'ta Xvfb, Windows'ta masaüstü oturumu.
- Her kaynak dosyanın dilden bağımsız açıklaması `docs/kod/` altında (olası Python taşıması için);
  dosyanın davranışı değişirse ilgili belgeyi de güncelle. Python geçiş rehberi `docs/PYTHON_GECIS.md`,
  iskelet `docs/python-iskelet/` (`pytest -q`); sunucu sözleşmesi (uçlar, JSON alanları) değişirse iskeleti de güncelle.
