# divan/src/web/tools.rs

**Amaç:** SW-10.9 — ulak ikilisinin dağıtımı. CI şablonu her koşumda buradan indirir; sunucu
güncellenince runner'lar ertesi koşumda yenisini kullanır.

`GET /api/tools/ulak?os=linux|windows` (giriş gerektirmez; `os` yoksa sunucunun sistemi):
- Dosya adları: `ulak-linux`, `ulak-windows.exe`. Başka `os` → 400.
- Aranan yerler sırayla: `ULAK_DIR/<ad>`, `DATA_DIR/tools/<ad>`, aynı sistemse sunucu ikilisinin
  yanındaki `ulak` / `ulak.exe`.
- Akış halinde, `Content-Disposition: attachment`, `X-Ulak-Version: <sunucu sürümü>`.
- Bulunamazsa 404 + WARN.
