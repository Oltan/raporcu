# rehearse/src/lib.rs

**Amaç:** Prova (`rehearse`) ve smoke (`e2esmoke`) için ortak yardımcılar. Gerçek ikilileri gerçek
süreç olarak çalıştırır; birim testi değil, kara kutu doğrulama.

| Öğe | Ne yapar |
|-----|----------|
| `workspace_root()`, `bin_dir()`, `exe(ad)` | depo kökü, derlenmiş ikili klasörü, `ad` + platform uzantısı |
X
| `free_port()` | 127.0.0.1:0'a bağlanıp boş port alır |
| `Server` | `raporcu`'yu geçici `DATA_DIR`, boş port, `CI_API_TOKEN=TOKEN`, `DRY_RUN=true`, `LOG_STDOUT=false` ile başlatır; `/api/health` 200 olana kadar (20 sn) bekler; `Drop`'ta öldürür. `extra_env` ek değişkenler |
| `get`, `get_with`, `get_bytes`, `get_json`, `post` | yönlendirme izlemeyen (max_redirects 0) ureq istemcisi; HTTP hatası değil durum kodu döner |
| `copy_dir(src, dst)` | özyinelemeli kopya (`target`, `.stajci` hariç) |
| `sample_dir()` | `testdata/allure/ornek-giris` |
| `Stajci` | `stajci` ikilisini proje klasöründe sahte GitLab değişkenleriyle (`CI_PROJECT_PATH=prova/ornek-test-projesi`, pipeline 777…, `STAJ_RETRY_BASE_MS=200`) çalıştırır; `set(k, v)`, `command`, `run → Output {code, text}` |
| `Checks` | `check(id, ne, ok, ayrıntı)` → `[GEÇTİ] SW-x.y …` / `[KALDI] …`; `result()` özet + çıkış kodu (0/1) |

## Python karşılığı
`subprocess.Popen` + `requests` (`allow_redirects=False`); `Checks` basit bir sınıf.
