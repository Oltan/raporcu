# divan/src/web/mod.rs

**Amaç:** Sayfa ve salt-okur API yönlendirmeleri, statik dosyalar, ortak hata/şablon yardımcıları.

## Yollar (giriş gerektirmez; kapalı ağ — admin işlemleri hariç)
| Yol | İşleyici | Not |
|-----|----------|-----|
| `GET /` | pages::dashboard | `?project=&branch=&page=` |
| `GET /runs/{id}`, `/runs/{id}/live` | rapor, canlı ekran | |
| `GET /canli` | etkin koşumu bekleyen canlı giriş sayfası | |
| `GET /runs/{id}/doors.csv`, `.dxl` | DOORS dışa aktarma | |
| `GET /ws/runs/{id}` | live::ws | WebSocket |
| `GET /scenarios/{id}` | senaryo detayı | |
| `GET /history?project=&branch=&key=` | senaryo geçmişi | |
| `GET /feature?project=&branch=&name=` | feature geçmişi | |
| `GET /search`, `/export.csv` | arama, CSV | |
| `GET /healing` | self-healing paneli | |
| `GET /s/{token}`, `/s/{token}/scenarios/{id}`, `/s/{token}/files/{id}` | paylaşım | |
| `GET /admin`; `POST /admin/runs/{id}/share`, `/admin/shares/{id}/revoke`, `/admin/scenarios/{id}/triage`, `/admin/scenarios/{id}/jira`, `/admin/runs/{id}/fix` | admin (giriş ister) | |
| `GET/POST /login`, `POST /logout`, `GET /api/admin/me` | giriş | |
| `GET /static/{yol}` | ikiliye gömülü dosyalar (`static/`), MIME tahmini, 1 saat önbellek | |
| `GET /files/{id}` | ek dosyası (range destekli) | |
| `GET /api/tools/ulak?os=linux\|windows` | ulak dağıtımı | |
| `GET /api/canli`, `/api/runs`, `/api/runs/{id}`, `/api/runs/{id}/live`, `/api/scenarios/{id}` | JSON | |
| `GET /api/health` | `{"durum":"ok","surum":"0.1.0"}` | |

## JSON API biçimi
- `/api/runs?project=&branch=&page=` → `{"sayfa":1,"kosumlar":[RunRow…]}` (50'lik).
- `/api/runs/{id}` → `{"kosum":RunRow,"senaryolar":[{"senaryo":ScenarioRow,"ekler":[…],"deneme_sayisi":n}]}`.
- `/api/scenarios/{id}` → `{"senaryo","adimlar","ekler","denemeler"}`.
- `/api/runs/{id}/live` → canlı anlık görüntü (`live.md`).
- `/api/canli` → `{"run_id":n}`, etkin koşum yoksa `{"run_id":null}` (`pages.md`).

## Yardımcılar
- `AppError(durum, mesaj)`: kullanıcıya kısa mesaj; `anyhow` hataları 500 "Sunucu hatası" + ERROR log.
- `render(şablon)`: HTML; şablon hatası 500.
