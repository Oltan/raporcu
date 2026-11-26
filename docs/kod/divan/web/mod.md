# divan/src/web/mod.rs

**Amaç:** Sayfa ve salt-okur API yönlendirmeleri, statik dosyalar, ortak hata/şablon yardımcıları.

## Yollar
İki grup (`routes()` ve `ekip_routes()`); `lib.rs` ekip grubuna `auth::ekip_guard` ara katmanını takar.

**Giriş istemeyen** (`routes()`):
| Yol | İşleyici | Not |
|-----|----------|-----|
| `GET /?yazilim=` | genel::ana_sayfa | dışa açık ana sayfa (`genel.md`) |
| `GET /rapor/{id}`, `/rapor/{id}/test/{sid}`, `/rapor/{id}/dosya/{aid}` | yayınlanmış sürüm raporu | |
| `GET /g/{*}` | 0.1.x genel linkleri → `/` | |
| `GET /s/{token}`, `/s/{token}/scenarios/{id}`, `/s/{token}/files/{id}` | ekip linki (token); eski `genel` görünüm → `/` | |
| `GET /admin`, `/admin/yayinlar*`, `/admin/ayarlar`; `POST /admin/...` | admin (`Admin` çıkarıcısı giriş ister) | `yayinlar.md` |
| `GET/POST /login`, `POST /logout`, `GET /api/admin/me` | giriş (ekip ya da admin şifresi) | |
| `GET /static/{yol}` | ikiliye gömülü dosyalar (`static/`), MIME tahmini, 1 saat önbellek | |
| `GET /api/tools/ulak?os=linux\|windows` | ulak dağıtımı | |
| `GET /api/health` | `{"durum":"ok","surum":"0.2.0"}` | `lib.rs` |

**Ekip paneli** (`ekip_routes()`; `EKIP_SIFRESI` verilmişse ekip ya da admin oturumu):
| Yol | İşleyici | Not |
|-----|----------|-----|
| `GET /panel` | pages::dashboard | `?project=&branch=&page=` |
| `GET /runs/{id}`, `/runs/{id}/live` | rapor, canlı ekran | |
| `GET /canli` | etkin koşumu bekleyen canlı giriş sayfası | |
| `GET /runs/{id}/doors.csv`, `.dxl` | DOORS dışa aktarma | |
| `GET /ws/runs/{id}` | live::ws | WebSocket |
| `GET /scenarios/{id}` | senaryo detayı | |
| `GET /history`, `/feature`, `/search`, `/export.csv`, `/healing` | geçmiş, arama, CSV, self-healing | |
| `GET /files/{id}` | ek dosyası (range destekli) | |
| `GET /api/canli`, `/api/runs`, `/api/runs/{id}`, `/api/runs/{id}/live`, `/api/scenarios/{id}` | JSON | |

## JSON API biçimi
- `/api/runs?project=&branch=&page=` → `{"sayfa":1,"kosumlar":[RunRow…]}` (50'lik).
- `/api/runs/{id}` → `{"kosum":RunRow,"senaryolar":[{"senaryo":ScenarioRow,"ekler":[…],"deneme_sayisi":n}]}`.
- `/api/scenarios/{id}` → `{"senaryo","adimlar","ekler","denemeler"}`.
- `/api/runs/{id}/live` → canlı anlık görüntü (`live.md`).
- `/api/canli` → `{"run_id":n}`, etkin koşum yoksa `{"run_id":null}` (`pages.md`).

## Yardımcılar
- `AppError(durum, mesaj)`: kullanıcıya kısa mesaj; `anyhow` hataları 500 "Sunucu hatası" + ERROR log.
- `render(şablon)`: HTML; şablon hatası 500.
