# ulak/src/client.rs

**Amaç:** divan HTTP istemcisi. Her istekte `Authorization: Bearer <CI_API_TOKEN>` ve
`x-ulak-version: <sürüm>`. Sistem proxy'si **kullanılmaz** (sunucu aynı ağda). Bağlantı zaman
aşımı 5 sn, toplam 600 sn. HTTP durum kodları hata sayılmaz, çağıran bakar.

- `post_json`, `post_bytes(yol, içerik_türü, bayt)`, `post_file(yol, dosya)` (zip'i akış halinde),
  `get` → `Reply {status, body}`.
- `with_retry(ne, f)`: ağ hatası ya da 5xx'te `ULAK_RETRY_MAX` (5) kez, `ULAK_RETRY_BASE_MS`
  (2000) ms'den başlayıp ikiye katlanan (en fazla 60 sn) bekleme; 4xx hemen döner.
- `ensure_ok(reply, ne)`: 2xx değilse hata (409 ise sunucunun mesajı aynen, ör. sürüm uyumsuz).

## Python karşılığı
`requests.Session()`, `trust_env=False`, `timeout=(5, 600)`; yeniden deneme döngüsü elle (urllib3 Retry 4xx/5xx ayrımını aynı yapmaz).
