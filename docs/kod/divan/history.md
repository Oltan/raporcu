# divan/src/history.rs

**Amaç:** SW-4.2, 4.6 — senaryo geçmişi ve flaky tespiti; **her zaman proje (ve branch) bazında**,
böylece iki projenin aynı adlı senaryoları karışmaz.

- `scenario_history(c, proje, branch?, key, limit, offset)`: o senaryonun koşumlardaki son
  denemeleri, en yeni koşum önce.
- `flaky_keys(c, proje, branch, pencere)`: son `pencere` (FLAKY_WINDOW, 10) koşumda **aynı commit'te
  kaldıktan sonra geçen** senaryolar: `failed`/`broken` bir denemeden sonra, aynı `commit_sha`'lı daha yeni
  bir koşumda ya da aynı koşumun daha sonraki denemesinde (SW-4.9) `passed`. Geçerken kalmaya başlayıp bir
  daha geçmeyen senaryo (bozulma) ve commit değişince geçen senaryo (düzeltme) flaky değildir. Commit
  boşsa koşumlar aynı sürüm sayılır.
- `is_flaky(...)`: tek senaryo için.

## Testler
`test_sw_4_2_dashboard_history_and_flaky_detection`, `test_sw_4_2_flaky_needs_pass_after_fail_on_same_commit`, `test_sw_4_6_project_created_from_gitlab_info_and_histories_separate`
