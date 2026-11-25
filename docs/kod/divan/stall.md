# divan/src/stall.rs

**Amaç:** SW-3.5 — yarıda kalan koşumlar.

- `mark_stalled(c, zaman_aşımı)`: `status='running'` ve `last_activity_at <= şimdi − zaman_aşımı`
  olan koşumların sayaçlarını hesaplar ve `interrupted` yapar (log WARN). `last_activity_at` her
  sonuç dosyasında, toplu yüklemede ve canlı olay grubunda tazelenir.
- `stall_loop`: zaman aşımının onda biri aralıkla (1–30 sn) çalışır; kapanan koşumlar için
  `hooks::after_finish`.

## Testler
`test_sw_3_5_stalled_run_becomes_interrupted`, `test_sw_3_5_finish_endpoint_marks_interrupted`.
