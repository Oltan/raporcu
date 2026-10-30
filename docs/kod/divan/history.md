# divan/src/history.rs

**Amaç:** SW-4.2, 4.6 — senaryo geçmişi ve flaky tespiti; **her zaman proje (ve branch) bazında**,
böylece iki projenin aynı adlı senaryoları karışmaz.

- `scenario_history(c, proje, branch?, key, limit, offset)`: o senaryonun koşumlardaki son
  denemeleri, en yeni koşum önce.
- `flaky_keys(c, proje, branch, pencere)`: son `pencere` (FLAKY_WINDOW, 10) koşumda **hem** son
  denemesi geçmiş bir koşumu **hem** de herhangi bir kalan/broken denemesi olan senaryolar. Bu
  tanım, aynı koşumda önce kalıp tekrar denemede geçeni de kapsar (SW-4.9 girdisi).
- `is_flaky(...)`: tek senaryo için.

## Testler
`test_sw_4_2_dashboard_history_and_flaky_detection`, `test_sw_4_6_project_created_from_gitlab_info_and_histories_separate`
