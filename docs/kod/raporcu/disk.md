# raporcu/src/disk.rs

**Amaç:** SW-10.8 — disk doluluğu uyarısı ve temizlik.

- `DiskProbe` arayüzü: `used_pct()`. Gerçek: `FsProbe(DATA_DIR)` = (toplam − kullanılabilir) / toplam.
  Testler sahte ölçüm verir.
- `DiskState.update(yüzde, uyarı_eşiği)`: eşik **ilk** aşıldığında WARN `disk doluluk eşiği aşıldı`;
  admin sayfalarında üst bant ("Disk %85 dolu…").
- `cleanup(c, paths, probe, uyarı=80, temizlik=90)`:
  - Doluluk temizlik eşiğinin altındaysa hiçbir şey yapmaz.
  - Önce **videolar** (`mime LIKE 'video/%'`), sonra **görüntüler**; her türde en eski koşumdan
    başlayarak. Her yeni koşuma geçerken yeniden ölçer; uyarı eşiğinin altına inince durur.
  - Ek silinince dosya gider, kayıt kalır: `deleted_at`, boyut ve neden. Senaryonun Jira bağlantısı
    (ya da Jira tag'i) varsa neden `Silindi, bkz. STAJ-42`, yoksa `Silindi (disk temizliği)`.
    Rapor sayfası silinen ekin yerine bu metni gösterir; dosya isteği 410.
  - Koşum, senaryo, adım, triage kayıtları ve metin ekleri silinmez. Log: `disk temizliği yapıldı`.
- `disk_loop`: 10 dakikada bir ölçüm + temizlik.

## Testler
`test_sw_10_8_cleanup_order_videos_then_images_oldest_first`, `test_sw_10_8_images_deleted_after_videos_when_still_full`.
