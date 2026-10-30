# ulak/src/ctx.rs

**Amaç:** Ortak bağlam, ortam değişkenleri, çıktı satırları.

## `Ctx::from_env()`
| Alan | Değişken | Varsayılan |
|------|----------|-----------|
| server | `ULAK_SERVER` | `http://127.0.0.1:8080` |
| token | `CI_API_TOKEN` | boş |
| project_dir | `ULAK_PROJECT_DIR` | çalışma klasörü |
| state_dir | `ULAK_STATE_DIR` | `<proje>/.ulak` |
| results_dir | `ULAK_RESULTS_DIR` (göreli ise projeye göre) | `<proje>/target/allure-results` |

Türetilen yollar: `.ulak/run.json` (`{run_id, report_url}`), `.ulak/stop` (izleyiciye bitti
sinyali), `.ulak/queue/` (gönderilemeyen zip'ler), `<proje>/ulak.env` (GitLab dotenv:
`ULAK_RUN_ID`, `ULAK_REPORT_URL`), `live_events_file()` = `ULAK_CANLI_DOSYA` ya da
`target/ulak-canli/olaylar.jsonl`.

`save_run` hem `run.json`'u hem `ulak.env`'i yazar.

## Diğer
- `say(mesaj)`: `[ulak run_id=12] mesaj` (run_id bilinmiyorsa `[ulak]`); CI çıktısı ile sunucu
  logu run_id ile eşleşir (SW-10.7).
- `ci_info(proje)`: `common::api::CiInfo`'yu GitLab değişkenlerinden doldurur; `CI_PROJECT_PATH`
  yoksa `ULAK_PROJECT` ya da `yerel/<klasör adı>`; branch yoksa `ULAK_BRANCH` ya da `yerel`;
  `os` = `ULAK_OS` ya da çalışan sistem.
- `split_command(s)`: kabuk kullanmadan komut satırını parçalar (boşluk, tek/çift tırnak, boş
  tırnak = boş argüman).

## Testler
`test_sw_1_1_split_command_without_shell`

## Python karşılığı
`os.environ.get`, `pathlib`; `split_command` yerine `shlex.split(s, posix=True)` (Windows yolları için ters bölüye dikkat).
