# ulak/src/main.rs

**Amaç:** CI aracının komut satırı. CI şablonu yalnızca bu alt komutları çağırır. Çıkış kodu
alt komutun döndürdüğüdür; beklenmeyen hata `[ulak] HATA: …` + çıkış 1.

| Alt komut | Seçenekler | Ne yapar |
|-----------|-----------|----------|
| `preflight` | | test projesini denetler (`preflight.md`) |
| `start` | `--no-tests` | koşumu açar, izleyiciyi başlatır, testleri koşar (`runner.md`) |
| `watch` | `--run-id N` (yoksa `.ulak/run.json`) | sonuçları ve canlı olayları gönderir (`watch.md`) |
| `upload` | `--resume`, `--queue <klasör>` | toplu zip gönderir; `--resume` kuyruktakileri (`upload.md`) |
| `finish` | `--status finished\|interrupted` | koşumu bitirir |
| `fix` | | OpenCode / Claude Code ile düzeltme (`fix.md`) |
| `--version` | | sürüm |

## Python karşılığı
`argparse` alt komutları; `sys.exit(kod)`. Çıkış kodları (0, 1, 2 sürüm, 3 bellek, 4 oturum) CI davranışını belirler, aynı kalmalı.
