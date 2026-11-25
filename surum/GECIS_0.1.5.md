# 0.1.4 → 0.1.5 geçiş rehberi

Daha eski bir sürümden geliyorsan önce `surum/GECIS_0.1.3.md`'yi uygula, sonra 0.1.5 ikililerini kur.
`GECIS_0.1.4.md`'deki `.gitlab-ci.yml` değişikliği 0.1.5 için de gerekir.

## 1. Ne değişti

| Değişiklik | Nerede |
|---|---|
| **Uygulama sürümü her yerde**, değer yoksa `—` | dashboard (ayrı **Uygulama** sütunu ve "son biten koşum" satırı), koşum sayfası başlığı, ekip linki, genel rapor linki, genel ana sayfa, gece e-postası (konu ve her rapor satırı) |
| `UYGULAMA_SURUMU` boşsa pom'daki sürüm | ulak `pom.xml`'deki `<uygulama.version>`'ı gönderir; schedule'da da sürüm görünür |
| İş logunda `uygulama sürümü: 1.5.0 (UYGULAMA_SURUMU)` | `test` işi, `ulak start` | 
| **Bağlı Jira issue'ları raporda**: "Jira issue'ları (N)" kartı | koşum sayfası (Jira linki, tür, atanan, senaryolara link); ekip linki (anahtar, başlık, durum; link ve atanan yok); genel rapor linki (yalnızca ürün hatalarınınki); gece e-postası |

Veritabanı ve `.env` değişmedi.

## 2. Önce: sahadaki sürümü kontrol et

Sürümü doldurduğun hâlde hiçbir yerde göremediysen büyük ihtimalle sunucu ya da runner'ın indirdiği
ulak 0.1.3'ten eski:

```bash
curl -s http://<divan>:8080/api/health          # "surum":"0.1.x"
```

GitLab'da son `test` işinin logunda `uygulama sürümü:` satırını ara. Satır yoksa runner eski ulak'ı
indiriyor demektir: sunucudaki `tools/ulak-linux` (ve `ulak-windows.exe`) eski kalmış.

## 3. Sunucu

```bash
sudo systemctl stop divan
sudo cp /opt/divan/divan /opt/divan/divan-0.1.4
sudo install -m 755 divan /opt/divan/divan
sudo install -m 755 ulak /var/lib/divan/tools/ulak-linux
sudo install -m 644 ulak.exe /var/lib/divan/tools/ulak-windows.exe   # Windows runner varsa
sudo chown divan:divan /opt/divan/divan /var/lib/divan/tools/*
sudo systemctl start divan
curl -s http://127.0.0.1:8080/api/health          # "surum":"0.1.5"
```

Windows sunucuda: divan'ı durdur, `divan.exe`'yi ve `veri\tools\ulak-windows.exe`'yi değiştir, başlat.

## 4. Kontrol

GitLab → Build → Pipelines → Run pipeline, `UYGULAMA_SURUMU=1.5.0`, `ULAK_TAGS` ile tek senaryo:

- [ ] `test` işinin logunda `uygulama sürümü: 1.5.0 (UYGULAMA_SURUMU)`
- [ ] dashboard → Koşumlar tablosunda **Uygulama** sütununda `1.5.0`
- [ ] koşum sayfasının başlığında `Uygulama 1.5.0`
- [ ] `@ABC-123` tag'li ya da bağlı bir senaryo varsa koşum sayfasında "Jira issue'ları" kartı:
      başlık ve durum (koşum bittikten birkaç saniye sonra Jira'dan gelir)
- [ ] admin → "Ekip linki" ile açılan sayfada da kart var, Jira'ya link yok
- [ ] `UYGULAMA_SURUMU`'yu boş bırakınca pom'daki sürüm görünüyor (logda `(pom.xml)`)

## 5. Geri dönüş

```bash
sudo systemctl stop divan
sudo cp /opt/divan/divan-0.1.4 /opt/divan/divan
sudo systemctl start divan
```
