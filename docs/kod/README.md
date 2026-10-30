# Kod belgeleri — dosya başına

Her kaynak dosyanın ne yaptığı, davranış kuralları, uç noktaları, ortam değişkenleri ve testleri.
Belgeler **dilden bağımsız** yazıldı: Rust'tan Python'a (ya da başka dile) geçilirse bu belgeler
ve aşağıdaki kabul testleri yeni kodun şartnamesidir. Kütüphane seçimi
gerektiren belgelerde ayrıca "Python karşılığı" notu var; genel eşleme aşağıdaki tabloda.

## Dizin

| Bölüm | Belgeler |
|-------|----------|
| Ortak (`crates/common`) | [lib](common/lib.md) · [allure](common/allure.md) — Allure dosya ayrıştırma · [api](common/api.md) — CI API tipleri, sürüm uyumu |
| Sunucu çekirdek (`crates/raporcu/src`) | [main](raporcu/main.md) · [lib](raporcu/lib.md) (AppState, router) · [config](raporcu/config.md) · [paths](raporcu/paths.md) · [db](raporcu/db.md) · [logging](raporcu/logging.md) · [auth](raporcu/auth.md) |
| Sonuç alma | [upload](raporcu/upload.md) (zip) · [ingest](raporcu/ingest.md) (Allure → veritabanı, is_latest) · [hooks](raporcu/hooks.md) (yükleme/bitiş sonrası işler) · [stall](raporcu/stall.md) (durgun koşum) · [live](raporcu/live.md) (canlı durum) |
| Rapor | [queries](raporcu/queries.md) · [history](raporcu/history.md) · [charts](raporcu/charts.md) (SVG) · [export](raporcu/export.md) (CSV, DOORS) · [share](raporcu/share.md) · [disk](raporcu/disk.md) |
| Entegrasyon | [triage](raporcu/triage.md) · [fix](raporcu/fix.md) (self-healing kayıtları) · [jira](raporcu/jira.md) · [gitlab](raporcu/gitlab.md) · [email](raporcu/email.md) |
| Web katmanı (`src/web`) | [mod](raporcu/web/mod.md) (yönlendirme, statik, WebSocket) · [ci](raporcu/web/ci.md) (CI uçları) · [files](raporcu/web/files.md) · [gitlab_hook](raporcu/web/gitlab_hook.md) · [tools](raporcu/web/tools.md) · [admin](raporcu/web/admin.md) · [pages](raporcu/web/pages.md) |
| Şablonlar ve statik | [templates](raporcu/templates/README.md) · [static](diger/static.md) |
| Veritabanı şeması | [migrations](diger/migrations.md) |
| CI aracı (`crates/stajci`) | [main](stajci/main.md) · [ctx](stajci/ctx.md) · [client](stajci/client.md) · [gherkin](stajci/gherkin.md) · [preflight](stajci/preflight.md) · [runner](stajci/runner.md) (`start`) · [watch](stajci/watch.md) · [upload](stajci/upload.md) (`upload`, `finish`) · [fix](stajci/fix.md) |
| Doğrulama | [tests](raporcu/tests/README.md) (entegrasyon testleri) · [rehearse lib](rehearse/lib.md) · [rehearse main](rehearse/main.md) (CI provası) · [e2esmoke](e2esmoke/main.md) |
| Test projesi ve CI | [Java test projesi](diger/java-test-projesi.md) · [CI şablonu ve dağıtım](diger/ci-ve-dagitim.md) |

Konu belgeleri: [anlık aktarım](../ANLIK_AKTARIM.md) · [CI kurulumu](../GITLAB_CI_KURULUMU.md) ·
[kod ajanı + vLLM](../KOD_AJANI_VLLM.md) · [test projesi gereksinimleri](../TEST_PROJESI_GEREKSINIMLERI.md) ·
[kararlar](../KARARLAR.md).

## Python'a geçiş haritası

Adım adım rehber: [`../PYTHON_GECIS.md`](../PYTHON_GECIS.md); çalışan iskelet: [`../python-iskelet/`](../python-iskelet/).

| Rust | Python karşılığı | Not |
|------|------------------|-----|
| axum + tokio | **FastAPI** (ya da Starlette) + uvicorn | uç noktalar ve yollar aynı kalmalı (stajci ve CI şablonu bunlara bağlı) |
| axum WebSocket | FastAPI `WebSocket` | `/ws/runs/{id}` aynı JSON anlık görüntüyü göndermeli; `live.html` değişmez |
| askama şablonları | **Jinja2** (`autoescape=True`) | sözdizimi neredeyse aynı; farklar `templates/README.md` |
| rusqlite (bundled) | `sqlite3` (stdlib) | `migrations/*.sql` aynen; mevcut `raporcu.db` taşınmadan açılır |
| rust-embed | `importlib.resources` / paket verisi | |
| serde / serde_json | `pydantic` ya da `dataclasses` + `json` | Allure alanları `common/allure.md` |
| argon2 | `argon2-cffi` | hash biçimi PHC (`$argon2id$…`) → mevcut `.env` hash'i geçerli kalır |
| zip | `zipfile` | `../` ve mutlak yol reddi (`upload.md`) |
| ureq (stajci) | `requests` | proxy kapalı: `session.trust_env = False` |
| clap (stajci) | `argparse` | alt komut ve çıkış kodları aynı kalmalı (CI şablonu bağlı) |
| stajci tek ikili | **PyInstaller** `--onefile` (Linux ve Windows ayrı derlenir) | `/api/tools/stajci?os=` aynı dosya adlarıyla sunar |
| lettre (SMTP) | `smtplib` | |
| tracing JSON log | `logging` + JSON biçimleyici | `run_id` alanı her satırda (SW-10.7) |

**Kabul ölçütü:** Python sürümü, Rust ikilileri yerine konduğunda `cargo run -p rehearse` ve
`cargo run -p e2esmoke` (dışarıdan süreç olarak sınarlar; `STAJ_SERVER` ile kurulu sunucuya karşı)
geçmelidir. `crates/raporcu/tests/*` davranış tanımı olarak `pytest`'e taşınır.

**Dikkat edilecek bellek hedefi:** sunucu < 100 MB (SW-10.6). Python + FastAPI ~60–80 MB ile başlar;
zip yükleme akış halinde (`SpooledTemporaryFile`) işlenmeli, sonuçlar belleğe toplanmamalıdır.
