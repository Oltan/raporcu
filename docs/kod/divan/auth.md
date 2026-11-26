# divan/src/auth.rs

**Amaç:** SW-9.1 CI token koruması, SW-9.5 admin girişi, SW-10.9 ulak sürüm kontrolü.

## CI koruması (`ci_guard`, `/api/ci/*` için ara katman)
1. Token: `Authorization: Bearer <t>` ya da `X-Ci-Token: <t>`. `CI_API_TOKEN` ayarlı değilse ya da
   token farklıysa **401** "CI_API_TOKEN eksik ya da yanlış" (+WARN log, yol ile).
   Karşılaştırma sabit zamanlı.
2. Sürüm: `X-Ulak-Version` varsa ve sunucuyla major.minor uyuşmuyorsa **409**
   "ulak sürümü uyumsuz: ulak X, sunucu Y. Yeni ulak'ı <PUBLIC_URL>/api/tools/ulak
   adresinden indirin." Başlık yoksa geçer.

## Admin girişi
- `Authenticator` arayüzü: `verify(kullanıcı, şifre) -> bool`. Şimdilik tek uygulama `LocalAdmin`:
  kullanıcı adı `admin`, şifre `.env`'deki argon2 hash'e göre. AD/LDAP ileride ikinci uygulama.
- `hash_password` (argon2id, rastgele tuz). `update_env_file(yol, anahtar, değer)`: `.env`'de
  anahtarı değiştirir ya da ekler, diğer satırları korur; değer tek tırnak içinde (`$` için).
  `divan set-admin-password` bunu kullanır.
- `Sessions`: bellekte `token → bitiş` (12 saat); token 64 hex (iki UUIDv4). Süresi geçenler
  oluştururken temizlenir.
- Çerez: `divan_oturum=<token>; HttpOnly; SameSite=Strict; Path=/; Max-Age=43200`.
- `LoginThrottle`: IP başına başarısız deneme sayısı; her hatadan sonra bekleme 1, 2, 4 … en fazla
  60 sn. Beklemede giriş **429**. Başarılı girişte sıfırlanır.
- `Admin` çıkarıcısı: geçerli oturum çerezi yoksa → tarayıcı GET isteği (`Accept: text/html`)
  ise `/login?next=<yol>`'a yönlendirme, değilse **401**.
- `client_ip`: bağlantının uzak adresi (yoksa 127.0.0.1).

## Ekip paneli (0.2.0)
- `EKIP_SIFRESI` (`.env`, gizli; log ve hata mesajlarından çıkarılır). Verilmezse panel açık, açılışta WARN.
- Giriş sayfasında tek şifre kutusu: admin şifresi tutarsa admin oturumu, değilse ekip şifresi
  (sabit zamanlı karşılaştırma) tutarsa ekip oturumu: `AppState.ekip` (`Sessions`), çerez
  `divan_ekip=<token>; HttpOnly; SameSite=Lax`. Aynı `LoginThrottle`. Çıkış iki çerezi de siler.
- `ekip_mi(state, başlıklar)`: şifre yok ya da geçerli ekip/admin oturumu. Admin oturumu ekip sayılır,
  ekip oturumu admin sayılmaz.
- `ekip_guard`: ekip paneli uçlarının ara katmanı; girişsiz tarayıcı GET → `/login?next=<yol+sorgu>`,
  diğerleri **401** "Ekip girişi gerekli".

## Testler
`test_sw_9_5_argon2_verify`, `test_sw_9_5_env_file_update_keeps_other_lines`,
`test_sw_9_5_throttle_grows`; HTTP düzeyi: `tests/temel.rs`, `tests/rapor.rs` (`test_sw_9_5_ekip_paneli_sifreyle`).

## Python karşılığı
`argon2-cffi` (`PasswordHasher`), `hmac.compare_digest`, FastAPI `Depends` ile CI/admin korumaları.
