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
  `os` = `ULAK_OS` ya da çalışan sistem. `app_version` = `uygulama_surumu(proje)`.
- `uygulama_surumu(proje) -> (sürüm?, kaynak)` (SW-4.3): sıra `ULAK_UYGULAMA_SURUMU`, `UYGULAMA_SURUMU`
  (Run pipeline formu, schedule ya da proje CI değişkeni), `ULAK_EXTRA_ARGS`'taki `-D<özellik>=X` (son
  geçen), `pom.xml`'deki ilk `<özellik>X</özellik>` (`${…}` sayılmaz). Özellik `ULAK_UYGULAMA_OZELLIGI`,
  varsayılan `uygulama.version`. `uygulama_surumu_satiri`: `ulak start`'ın yazdığı
  `uygulama sürümü: X (kaynak)` ya da `uygulama sürümü: yok (…)` satırı.
- `split_command(s)`: kabuk kullanmadan komut satırını parçalar (boşluk, tek/çift tırnak, boş
  tırnak = boş argüman).

## Testler
`test_sw_1_1_split_command_without_shell`

## Python karşılığı
`os.environ.get`, `pathlib`; `split_command` yerine `shlex.split(s, posix=True)` (Windows yolları için ters bölüye dikkat).

## Ayar okuma
- `env(ad)`: boşlukları kırpar, boş değer yok sayılır. Tam ad yoksa büyük harfe çevrilip `-` → `_`
  yapılınca aynı olan değişken (`ulak-fix-max`, `Ulak_Fix_Max`) okunur ve bir kez uyarılır.
- `sayi_ayari(ad, varsayılan)` → (değer, kaynak); `sinir_ayari(ad)` → (`None` = sınırsız, log yazısı).
  Sayı olmayan değerde uyarı.
