# divan/src/web/ci.rs

**Amaç:** `/api/ci/*` uçları (CI_API_TOKEN ve sürüm kontrolü `auth::ci_guard`'da).

| Uç | Gövde | Davranış | Yanıt |
|----|-------|----------|-------|
| `POST /runs?<CiInfo sorgusu>` | zip | `project_path` zorunlu (400). Yeni koşum, zip işlenir, `finished` yapılır; sonra `after_results` + `after_finish` | `IngestSummary` |
| `POST /runs/{id}/upload` | zip | Var olan (canlı) koşuma son toplu zip; tekrarlar sayılmaz; koşum durumu değişmez | `IngestSummary` |
| `POST /runs/start` | JSON: CiInfo alanları + `total?` + `expected: [{name, feature}]` | `running` koşum; `total` yoksa `expected` sayısı; beklenenler sırayla kaydedilir | `{run_id, report_url}` |
| `POST /runs/{id}/results?name=<dosya>` | tek Allure dosyası | Güvensiz ad 400, koşum yoksa 404. Ek → `runs/<id>/files/` (`.part` + rename, varsa atla), `last_activity` tazelenir. json → transaction içinde işle, bağla, yeniden say | `IngestSummary` |
| `POST /runs/{id}/events` | JSON dizisi (≤5000) | Koşum `running` değilse 202 (yok sayılır). `last_activity` tazelenir, olaylar `canli`'ya uygulanır, canlı yayın | 204 |
| `POST /runs/{id}/finish` | `{status?}` | `finished` / `interrupted` dışı 400; yeniden sayar, bitirir, `after_finish` | RunCounts |
| `GET /runs/{id}/fix-candidates` | | `fix::candidates` | `[FixCandidate]` |
| `POST /runs/{id}/fix-result` | `FixResult` | `fix::record`, canlı yayın | `{id, report_url}` |
| `POST /runs/{id}/fix-started` | `{scenario_key}` | yalnızca log: `OpenCode denemesi başladı` | `{ok:true}` |

Zip yollarında log: başarıda `yükleme alındı` (run_id, proje, branch, dosya sayısı, sonuç, tekrar,
boyut, süre ms), hatada WARN `yükleme reddedildi` (neden, HTTP durumu). Hata yanıtı
`{"hata": "<neden>"}`. `report_url(koşum)` = `<PUBLIC_URL>/runs/<id>`.
