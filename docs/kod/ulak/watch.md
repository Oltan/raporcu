# ulak/src/watch.rs

**Amaç:** `ulak watch` (SW-3.2) — test koşarken sonuçları ve canlı olayları anında gönderir.
Test işinden ayrı süreç; çökerse test etkilenmez. Ayrıntılı akış: `docs/ANLIK_AKTARIM.md`.

## Döngü (her `ULAK_WATCH_INTERVAL_MS`, 1000)
1. **Canlı olaylar (`EventTail`):** olay dosyasını kayıtlı bayt konumundan (en fazla 1 MB) okur;
   yalnızca son `\n`'e kadar olan tam satırlar; geçersiz JSON satırı atlanır; dizi olarak
   `POST /api/ci/runs/{id}/events`. Ağ hatası ya da 5xx → konum ilerlemez, tur başarısız sayılır.
   Dosya küçüldüyse (yeniden oluşturulmuş) konum 0. Dosya yoksa sorun değil.
2. **Allure dosyaları:** `allure-results`'taki gönderilmemiş, `Other` olmayan dosyalar; **ekler
   önce**, sonra json'lar (ad sırasıyla). Yazımı süren dosya atlanır: 300 ms'den yeni ve boyutu
   önceki turdan farklıysa (durma anında atlanmaz). `POST …/results?name=<ad>` ham baytlar.
   - 2xx → `sent.txt`'e eklenir. 4xx → mesaj basılır, gönderilmiş sayılır (tekrar denenmez).
     5xx/ağ hatası → tur durur, sonra tekrar.
3. Başarısız turdan sonra bekleme 1 sn'den ikiye katlanarak en fazla 30 sn.
4. `stop` dosyası varsa (ya da `ULAK_WATCH_MAX_SECS`, 6 saat, dolduysa) son tur ve çıkış 0.
   Gönderilemeyenler report işindeki toplu yüklemeyle gider.

`sent.txt` başlangıçta okunur; yeniden başlayan izleyici aynı dosyayı tekrar göndermez (gönderse
de sunucu saymaz).

## Testler
`test_sw_3_2_event_tail_sends_only_complete_lines`; prova SW-3.2, 3.4, 2.6.

## Python karşılığı
Döngü + `time.sleep`; dosya sonu takibi `f.seek(offset)`, `data.rfind(b"\n")`; gönderilenler `set` + `sent.txt`.
