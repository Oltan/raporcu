# Python iskeleti

divan'ın Python'a taşınması için çalışan başlangıç kodu. Rehber: [`../PYTHON_GECIS.md`](../PYTHON_GECIS.md).

| Dosya | Rust karşılığı | Kapsam |
|-------|----------------|--------|
| `divan/allure.py` | `crates/common/src/allure.rs` | tam |
| `divan/db.py` | `crates/divan/src/db.rs` | bağlantı, migration'lar (yedek yok) |
| `divan/defterdar.py` | `defterdar.rs` | tam |
| `divan/upload.py` | `upload.rs` | tam (akış halinde, bellek ölçüldü) |
| `divan/live.py` | `live.rs` | olaylar ve yayın tam |
| `divan/app.py` | `lib.rs`, `web/ci.rs`, `auth.rs` (CI kısmı) | sağlık, CI uçları (fix uçları hariç), canlı API, WebSocket; `snapshot` basitleştirilmiş |
| `ulak/watch.py` | `crates/ulak/src/watch.rs` | tam |
| `tests/test_iskelet.py` | `crates/divan/tests/*` | 10 test, gerçek `testdata/` ile |

```bash
python -m venv .venv && . .venv/bin/activate
pip install -r requirements.txt
pytest -q
```

Elle denemek için:

```bash
python -c "import uvicorn; from divan.app import create_app; uvicorn.run(create_app({'data_dir': 'veri', 'public_url': 'http://127.0.0.1:8080', 'ci_api_token': 'deneme', 'migrations_dir': '../../migrations', 'max_upload_mb': 512, 'max_entry_mb': 256}), port=8080)"
```

Bu sunucuya Rust `ulak upload` ile sonuç gönderilebilir (`ULAK_SERVER=http://127.0.0.1:8080
CI_API_TOKEN=deneme`). `/api/runs/{id}/live` çalışır; HTML sayfaları henüz yok (rehber §6).
