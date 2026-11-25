# divan/src/lib.rs

**Amaç:** Uygulama durumu, yönlendirici, arka plan görevleri.

## `AppState`
`cfg`, `paths`, `db`, `auth` (Authenticator), `sessions`, `throttle`, `recent_logs` (son WARN/ERROR),
`live` (koşum id yayını, kapasite 256), `disk` (doluluk durumu), `canli` (şu an koşan senaryolar).
`AppState::new(cfg, recent)` veritabanını açar.

## Yönlendirici (`router`)
- `/api/health` → `{"durum":"ok","surum":…}`
- `/api/gitlab/webhook` (POST, X-Gitlab-Token)
- `/api/ci/*` (CI token + sürüm ara katmanı, gövde sınırı MAX_ENTRY_MB):
  `ping`, `POST runs` (zip, yeni koşum), `POST runs/start`, `POST runs/{id}/upload` (zip, var olan),
  `POST runs/{id}/results?name=`, `POST runs/{id}/events`, `POST runs/{id}/finish`,
  `GET runs/{id}/fix-candidates`, `POST runs/{id}/fix-result`, `POST runs/{id}/fix-started`
- sayfalar ve salt-okur API: `web/mod.rs`

## `spawn_background`
zaman aşımı döngüsü, disk döngüsü, gecelik yedek döngüsü, saatlik log temizliği.
