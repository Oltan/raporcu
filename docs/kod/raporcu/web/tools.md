# raporcu/src/web/tools.rs

**Amaç:** SW-10.9 — stajci ikilisinin dağıtımı. CI şablonu her koşumda buradan indirir; sunucu
güncellenince runner'lar ertesi koşumda yenisini kullanır.

`GET /api/tools/stajci?os=linux|windows` (giriş gerektirmez; `os` yoksa sunucunun sistemi):
- Dosya adları: `stajci-linux`, `stajci-windows.exe`. Başka `os` → 400.
- Aranan yerler sırayla: `STAJCI_DIR/<ad>`, `DATA_DIR/tools/<ad>`, aynı sistemse sunucu ikilisinin
  yanındaki `stajci` / `stajci.exe`.
- Akış halinde, `Content-Disposition: attachment`, `X-Stajci-Version: <sunucu sürümü>`.
- Bulunamazsa 404 + WARN.
