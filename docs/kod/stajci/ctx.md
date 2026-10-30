# stajci/src/ctx.rs

**Amaç:** Ortak bağlam, ortam değişkenleri, çıktı satırları.

## `Ctx::from_env()`
| Alan | Değişken | Varsayılan |
|------|----------|-----------|
| server | `STAJ_SERVER` | `http://127.0.0.1:8080` |
| token | `CI_API_TOKEN` | boş |
| project_dir | `STAJ_PROJECT_DIR` | çalışma klasörü |
| state_dir | `STAJ_STATE_DIR` | `<proje>/.stajci` |
| results_dir | `STAJ_RESULTS_DIR` (göreli ise projeye göre) | `<proje>/target/allure-results` |

Türetilen yollar: `.stajci/run.json` (`{run_id, report_url}`), `.stajci/stop` (izleyiciye bitti
sinyali), `.stajci/queue/` (gönderilemeyen zip'ler), `<proje>/stajci.env` (GitLab dotenv:
`STAJ_RUN_ID`, `STAJ_REPORT_URL`), `live_events_file()` = `STAJ_CANLI_DOSYA` ya da
`target/stajci-canli/olaylar.jsonl`.

`save_run` hem `run.json`'u hem `stajci.env`'i yazar.

## Diğer
- `say(mesaj)`: `[stajci run_id=12] mesaj` (run_id bilinmiyorsa `[stajci]`); CI çıktısı ile sunucu
  logu run_id ile eşleşir (SW-10.7).
- `ci_info(proje)`: `common::api::CiInfo`'yu GitLab değişkenlerinden doldurur; `CI_PROJECT_PATH`
  yoksa `STAJ_PROJECT` ya da `yerel/<klasör adı>`; branch yoksa `STAJ_BRANCH` ya da `yerel`;
  `os` = `STAJ_OS` ya da çalışan sistem.
- `split_command(s)`: kabuk kullanmadan komut satırını parçalar (boşluk, tek/çift tırnak, boş
  tırnak = boş argüman).

## Testler
`test_sw_1_1_split_command_without_shell`

## Python karşılığı
`os.environ.get`, `pathlib`; `split_command` yerine `shlex.split(s, posix=True)` (Windows yolları için ters bölüye dikkat).
