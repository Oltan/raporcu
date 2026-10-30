# divan/src/web/admin.rs

**Amaç:** SW-9.5 giriş/çıkış uçları.

- `GET /login?next=` — form. `next` yalnızca `/` ile başlayan ve `//` ile başlamayan yollar
  (açık yönlendirme yok), yoksa `/`.
- `POST /login` (form: `password`, `next`): IP yavaşlatmadaysa **429** + "N sn sonra tekrar
  deneyin". Doğrulama (argon2) bloklayan iş parçacığında. Başarı: oturum çerezi + `next`'e
  yönlendirme, log `admin girişi başarılı` (yalnızca IP). Hata: yavaşlatma artar, 300 ms bekleme,
  **401** + form, log WARN `admin girişi başarısız` (yalnızca IP). Şifre hiçbir yere yazılmaz.
- `POST /logout`: oturumu siler, çerezi temizler, `/`'a yönlendirir.
- `GET /api/admin/me`: oturum varsa "admin".
