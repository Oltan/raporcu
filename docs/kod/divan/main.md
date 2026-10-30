# divan/src/main.rs

**Amaç:** Komut satırı.

| Komut | Davranış |
|-------|----------|
| `divan` / `divan serve` | yapılandırmayı okur, DATA_DIR'i hazırlar, logger'ı kurar, tokio (2 işçi, en fazla 8 bloklayan iş parçacığı) ile sunucuyu `BIND_ADDR`'de başlatır; Ctrl+C ile düzgün kapanır |
| `divan set-admin-password` | yeni şifreyi iki kez (gizli) sorar ya da `ULAK_NEW_ADMIN_PASSWORD`'ten alır; en az 8 karakter; argon2 hash'i `ENV_FILE` (varsayılan `.env`) içine `ADMIN_PASSWORD_HASH='…'` olarak yazar |
| `divan backup` | hemen yedek alır, yolunu yazar |
| diğer | kullanım metni, çıkış 2 |
