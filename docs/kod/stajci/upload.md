# stajci/src/upload.rs

**Amaç:** `stajci upload` (SW-2.1, 2.6, 5.4) ve `stajci finish` (SW-3.5).

## upload
1. `run.json` varsa run_id ile çıktı. Sonuç klasörü yoksa UYARI (yine de boş zip gönderilir;
   sunucu 422 + "Allure eklentisi" mesajı döner).
2. `zip_results`: klasördeki dosyalar ada göre sıralı `allure-results/<ad>` olarak; `.json`/`.txt`
   deflate, diğerleri (görüntü, video) stored. `.stajci/sonuclar-<uuid>.zip`.
3. Hedef: `run.json` varsa `POST /api/ci/runs/<id>/upload` (canlı koşumun son toplu zip'i,
   tekrarlar sayılmaz), yoksa `POST /api/ci/runs?<CiInfo sorgusu>` (yeni koşum).
4. Yeniden denemeli gönderim.
   - 2xx: zip silinir; `run.json` + `stajci.env` (`STAJ_REPORT_URL`) yazılır; "yüklendi: N sonuç
     (M tekrar), K ek" ve **"Rapor: <url>"** (pipeline environment linki ve iş çıktısı, SW-5.4).
   - 4xx: sunucu mesajı (`hata` alanı), çıkış 1.
   - Ağ hatası: zip `.stajci/queue/<uuid>.zip` + `<uuid>.json` (`{path, zip, run_id}`), "Sonuçlar …
     saklandı (GitLab artifact). Sunucu geri gelince: stajci upload --resume", çıkış 1.

## upload --resume [--queue klasör]
Kuyruktaki her `.json` (ada göre sıralı) için kaydedilen hedefe zip gönderilir; başarıda iki dosya
silinir. Kuyruk boşsa "gönderilecek sonuç yok". Birisi başarısızsa çıkış 1.

## finish [--status]
`run.json` yoksa atlanır (0). `POST /api/ci/runs/<id>/finish`; başarıda "koşum finished: N
senaryo, X geçti, Y kaldı. Rapor: …"; hata UYARI + çıkış 1 (sunucu zaman aşımıyla kapatır).

## Testler
`test_sw_2_1_zip_results_contains_all_files`; prova SW-2.6, 5.4, 3.5.

## Python karşılığı
`zipfile.ZipFile(…, compression=ZIP_DEFLATED)`; görüntü/video için `zf.write(…, compress_type=ZIP_STORED)`; akış halinde gönderim `requests.post(data=open(zip, "rb"))`.
