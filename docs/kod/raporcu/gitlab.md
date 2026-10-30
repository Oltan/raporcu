# raporcu/src/gitlab.rs

**Amaç:** GitLab webhook olaylarını işleme (SW-3.6, 7.5) ve fix pipeline'ı tetikleme (SW-7.4).

- `apply_pipeline_event(c, json)`: `object_attributes.id/status/ref/source/url`, `project.id/
  path_with_namespace/web_url`. Proje yoksa oluşturulur. `pipelines` tablosuna
  `(proje, pipeline_id)` tekil upsert. Log: `webhook alındı`. Döner: (proje, pipeline, durum, kaynak).
- `apply_mr_event(c, json)`: `object_attributes.iid/state` → `merged`/`closed`/`opened`; aynı
  projedeki `fix_attempts.mr_iid` satırlarının `mr_state`'ini günceller (self-healing panelinde
  birleştirilen/reddedilen).
- `trigger_fix_pipeline(st, proje, ref, koşum)`: `POST <GITLAB_URL>/api/v4/projects/<id>/pipeline`,
  başlık `PRIVATE-TOKEN`, gövde `{ref, variables:[STAJ_FIX_ONLY=true, STAJ_RUN_ID=<koşum>]}`. CI
  şablonunda bu değişkenle yalnızca fix işi koşar. Hata metinleri gizli değerlerden temizlenir
  (SW-9.2), WARN log.
- `agent()`: HTTP istemcisi, 30 sn zaman aşımı, sistem proxy'si + NO_PROXY.

## Testler
`tests/entegrasyon.rs`: 3.6, 7.4, 7.5, 9.2.


Not: `trigger_fix_pipeline(st, proje, ref, koşum, senaryo?)` — senaryo verilirse değişkenlere `STAJ_FIX_SCENARIO_ID` eklenir (senaryo sayfasındaki düğme).
