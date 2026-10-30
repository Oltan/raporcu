# raporcu/src/paths.rs

**Amaç:** SW-2.4 — tüm çalışma verisinin yolları **tek yerde**. Başka modül yol kurmaz.
Testler geçici bir DATA_DIR kullanır.

## Yerleşim
```
DATA_DIR/
  raporcu.db (+ -wal, -shm)      SQLite
  backup/raporcu-YYYYMMDD-HHMMSSmmm.db
  logs/raporcu.YYYY-MM-DD.log    JSON satır logları
  tmp/upload-<uuid>.zip          gelen gövde (işlenince silinir)
  runs/<run_id>/files/<source>   Allure ekleri (source = Allure dosya adı)
  outbox/*.eml                   DRY_RUN e-postaları
  tools/stajci-linux, stajci-windows.exe
```

## Fonksiyonlar
`root, db, backup_dir, logs_dir, tmp_dir, runs_dir, run_dir(id), run_files_dir(id),
attachment(run_id, source), outbox_dir, tools_dir` — hepsi DATA_DIR altında yol döner.
`ensure()` — kök, backup, logs, tmp, runs, outbox, tools klasörlerini oluşturur.

## Testler
`test_sw_2_4_all_paths_under_data_dir`

## Python karşılığı
`pathlib.Path` dönen metotlar; `ensure` için `mkdir(parents=True, exist_ok=True)`.
