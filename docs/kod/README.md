# Kod belgeleri — dosya başına

Her kaynak dosyanın ne yaptığı, davranış kuralları, uç noktaları, ortam değişkenleri ve testleri.
Belgeler **dilden bağımsız** yazıldı: Rust'tan Python'a (ya da başka dile) geçilirse bu belgeler
ve aşağıdaki kabul testleri yeni kodun şartnamesidir. Kütüphane seçimi
gerektiren belgelerde ayrıca "Python karşılığı" notu var; genel eşleme aşağıdaki tabloda.

## Dizin

| Bölüm | Belgeler |
|-------|----------|
| Ortak (`crates/common`) | [lib](common/lib.md) · [allure](common/allure.md) — Allure dosya ayrıştırma · [api](common/api.md) — CI API tipleri, sürüm uyumu |
| Sunucu çekirdek (`crates/divan/src`) | [main](divan/main.md) · [lib](divan/lib.md) (AppState, router) · [config](divan/config.md) · [ayarlar](divan/ayarlar.md) (Jira şablonu, DOORS) · [paths](divan/paths.md) · [db](divan/db.md) · [logging](divan/logging.md) · [auth](divan/auth.md) |
| Sonuç alma | [upload](divan/upload.md) (zip) · [defterdar](divan/defterdar.md) (Allure → veritabanı, is_latest) · [hooks](divan/hooks.md) (yükleme/bitiş sonrası işler) · [stall](divan/stall.md) (durgun koşum) · [live](divan/live.md) (canlı durum) |
| Rapor | [queries](divan/queries.md) · [history](divan/history.md) · [charts](divan/charts.md) (SVG) · [export](divan/export.md) (CSV, DOORS) · [share](divan/share.md) · [disk](divan/disk.md) |
| Entegrasyon | [triage](divan/triage.md) · [fix](divan/fix.md) (self-healing kayıtları) · [jira](divan/jira.md) · [gitlab](divan/gitlab.md) · [email](divan/email.md) |
| Web katmanı (`src/web`) | [mod](divan/web/mod.md) (yönlendirme, statik, WebSocket) · [ci](divan/web/ci.md) (CI uçları) · [files](divan/web/files.md) · [gitlab_hook](divan/web/gitlab_hook.md) · [tools](divan/web/tools.md) · [admin](divan/web/admin.md) · [ayarlar](divan/web/ayarlar.md) · [pages](divan/web/pages.md) |
| Şablonlar ve statik | [templates](divan/templates/README.md) · [static](diger/static.md) |
| Veritabanı şeması | [migrations](diger/migrations.md) |
| CI aracı (`crates/ulak`) | [main](ulak/main.md) · [ctx](ulak/ctx.md) · [client](ulak/client.md) · [gherkin](ulak/gherkin.md) · [preflight](ulak/preflight.md) · [runner](ulak/runner.md) (`start`) · [watch](ulak/watch.md) · [upload](ulak/upload.md) (`upload`, `finish`) · [fix](ulak/fix.md) |
| Doğrulama | [tests](divan/tests/README.md) (entegrasyon testleri) · [rehearse lib](rehearse/lib.md) · [rehearse main](rehearse/main.md) (CI provası) · [e2esmoke](e2esmoke/main.md) |
| Test projesi ve CI | [Java test projesi](diger/java-test-projesi.md) · [CI şablonu ve dağıtım](diger/ci-ve-dagitim.md) |

Konu belgeleri: [anlık aktarım](../ANLIK_AKTARIM.md) · [CI kurulumu](../GITLAB_CI_KURULUMU.md) ·
[kod ajanı + vLLM](../KOD_AJANI_VLLM.md) · [test projesi gereksinimleri](../TEST_PROJESI_GEREKSINIMLERI.md) ·
[kararlar](../KARARLAR.md).

## Python'a geçiş haritası

Adım adım rehber: [`../PYTHON_GECIS.md`](../PYTHON_GECIS.md); çalışan iskelet: [`../python-iskelet/`](../python-iskelet/).

| Rust | Python karşılığı | Not |
|------|------------------|-----|
| axum + tokio | **FastAPI** (ya da Starlette) + uvicorn | uç noktalar ve yollar aynı kalmalı (ulak ve CI şablonu bunlara bağlı) |
| axum WebSocket | FastAPI `WebSocket` | `/ws/runs/{id}` aynı JSON anlık görüntüyü göndermeli; `live.html` değişmez |
| askama şablonları | **Jinja2** (`autoescape=True`) | sözdizimi neredeyse aynı; farklar `templates/README.md` |
| rusqlite (bundled) | `sqlite3` (stdlib) | `migrations/*.sql` aynen; mevcut `divan.db` taşınmadan açılır |
| rust-embed | `importlib.resources` / paket verisi | |
| serde / serde_json | `pydantic` ya da `dataclasses` + `json` | Allure alanları `common/allure.md` |
| argon2 | `argon2-cffi` | hash biçimi PHC (`$argon2id$…`) → mevcut `.env` hash'i geçerli kalır |
| zip | `zipfile` | `../` ve mutlak yol reddi (`upload.md`) |
| ureq (ulak) | `requests` | proxy kapalı: `session.trust_env = False` |
| clap (ulak) | `argparse` | alt komut ve çıkış kodları aynı kalmalı (CI şablonu bağlı) |
| ulak tek ikili | **PyInstaller** `--onefile` (Linux ve Windows ayrı derlenir) | `/api/tools/ulak?os=` aynı dosya adlarıyla sunar |
| lettre (SMTP) | `smtplib` | |
| tracing JSON log | `logging` + JSON biçimleyici | `run_id` alanı her satırda (SW-10.7) |

**Kabul ölçütü:** Python sürümü, Rust ikilileri yerine konduğunda `cargo run -p rehearse` ve
`cargo run -p e2esmoke` (dışarıdan süreç olarak sınarlar; `ULAK_SERVER` ile kurulu sunucuya karşı)
geçmelidir. `crates/divan/tests/*` davranış tanımı olarak `pytest`'e taşınır.

**Dikkat edilecek bellek hedefi:** sunucu < 100 MB (SW-10.6). Python + FastAPI ~60–80 MB ile başlar;
zip yükleme akış halinde (`SpooledTemporaryFile`) işlenmeli, sonuçlar belleğe toplanmamalıdır.
