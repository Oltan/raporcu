# 0.1.5 → 0.2.0 geçiş rehberi

0.2.0 **sözleşmeyi değiştirir** (yeni alan `UYGULAMA_ADI`): divan ve ulak birlikte güncellenir. Eski
(0.1.x) ulak'la gelen istek reddedilir (iş 2 koduyla düşer). Runner'lar yeni ulak'ı divan'dan
indirdiği için sunucudaki `tools/ulak-*` dosyalarını değiştirmek yeter.

Daha eski bir sürümden geliyorsan önce `GECIS_0.1.3.md`'deki sunucu, `.env`, jar ve test projesi
adımlarını, `GECIS_0.1.4.md`'deki `.gitlab-ci.yml` değişikliğini uygula.

## 1. Ne değişti

| Değişiklik | Senin için anlamı |
|---|---|
| ⚠ **`/` artık dışa açık rapor sayfası** | Dashboard **`/panel`** adresinde. Yer imlerini güncelle |
| ⚠ **Dashboard ekip şifresiyle** (`EKIP_SIFRESI`) | Panel, koşum/senaryo sayfaları, arama, canlı ekran giriş ister. Dışa açık sayfa ve yayınlanan raporlar istemez |
| **Rapor yayınlama** (Admin → Raporlar) | Rapor = repo + yazılım adı + sürüm. Seçtiğin koşumlar tek raporda birleşir; ayrı koşulan feature'lar tek rapor. İstemediğin testi çıkarırsın. Ana sayfada yalnızca yayınlananlar görünür |
| **`UYGULAMA_ADI`** (yeni CI değişkeni) | Aynı repo farklı tag'lerle farklı yazılım/konfigürasyon olarak koşulur; dışarıda her biri ayrı sekme ve rapor, panelde tek proje |
| **"İnceleniyor" kalktı** | Dışarıda yalnızca Geçti / Kaldı. İkinci denemede geçen test "Geçti" (eski hata: hiç görünmüyordu) |
| JUnit / ortam yoklama testleri dışarıda | `core.HeadlessEnvironmentProbe.*` gibi feature dosyası olmayan sonuçlar raporlara ve toplama girmez (panelde görünmeye devam eder) |
| Ana sayfa sadeleşti | Toplam / Geçen / Kalan / Başarı oranı; son rapor; **Sürümlere göre başarı**; **sürüm başına** sütunlar; rapor listesi. Bölüm tablosu, feature geçmişi, sonuç cümlesi, commit yok |
| Rapor sayfası | "Yazılım · sürüm" başlığı; feature ve test case'ler açılır/kapanır; Jira issue'su test case'in içinde; koşum no, commit, branch, feature sekmeleri yok |
| Eski linkler | 0.1.x "Genel ana sayfa linki" (`/g/...`) ve "Genel rapor linki" (`/s/...` genel) → `/`'e yönlenir. "Ekip linki" aynen çalışır |
| Dashboard feature tablosu | Son 10 koşumda koşulan **her** feature'ın en son sonucu (ayrı koşulanlar da görünür); "Yazılım" sütunu |

Veritabanı: migration `0009_yayin` açılışta kendiliğinden uygulanır (yeni tablolar + `runs.app_name`).
Öncesinde veri klasörünü yedekle.

## 2. Sunucu

```bash
sudo systemctl stop divan
sudo cp -a /var/lib/divan /var/lib/divan-yedek-$(date +%Y%m%d)
sudo cp /opt/divan/divan /opt/divan/divan-0.1.5
sudo cp /var/lib/divan/tools/ulak-linux /var/lib/divan/tools/ulak-linux-0.1.5
sudo install -m 755 divan /opt/divan/divan
sudo install -m 755 ulak /var/lib/divan/tools/ulak-linux
sudo install -m 644 ulak.exe /var/lib/divan/tools/ulak-windows.exe   # Windows runner varsa
sudo chown divan:divan /opt/divan/divan /var/lib/divan/tools/*
```

`/opt/divan/.env`'e ekle (ekibe vereceğin şifre; admin şifresinden farklı olsun):

```
EKIP_SIFRESI=<uzun bir şifre>
```

```bash
sudo systemctl start divan
curl -s http://127.0.0.1:8080/api/health                    # "surum":"0.2.0"
journalctl -u divan -n 100 | grep "migration uygulandı"     # 0009_yayin
```

Windows sunucuda: divan'ı durdur, veri klasörünü yedekle, `divan.exe` ile `veri\tools\ulak-windows.exe`'yi
değiştir, `.env`'e `EKIP_SIFRESI` ekle, başlat.

Runner makinesinde elle kopyalanmış bir ulak varsa (ör. `preflight` için) onu da değiştir.

## 3. Test projesinin `.gitlab-ci.yml`'ı

`variables:` altına (`UYGULAMA_SURUMU`'nun yanına):

```yaml
  UYGULAMA_ADI:
    value: ""
    description: "Test edilen yazılımın adı (dışa açık raporda sekme). Boş = repo adı. Ör: Kontrol Yazılımı"
```

Tam dosya: `surum/linux/ornek.gitlab-ci.yml`.

Aynı repoyu iki yazılım olarak koşturmak için iki schedule (Build → Pipeline schedules):
- `UYGULAMA_ADI=Kontrol Yazılımı`, `ULAK_TAGS=@kontrol`, `UYGULAMA_SURUMU=…`;
- `UYGULAMA_ADI=Simülatör`, `ULAK_TAGS=@sim`, `UYGULAMA_SURUMU=…`.

## 4. İlk rapor

1. GitLab → Run pipeline: `UYGULAMA_ADI`, `UYGULAMA_SURUMU`, istersen `ULAK_TAGS` doldur. Feature'ları ayrı
   ayrı koşturuyorsan hepsinde aynı ad ve sürümü ver.
2. divan → giriş (admin) → **Raporlar** (üst menü) → listede yazılım + sürüm → **Yayınla**.
3. Koşumlar işaretli gelir; istemediğin koşumun işaretini kaldır. Test listesinde rapora girmesin
   istediğin testleri "rapordan çıkar" ile işaretle (sonraki sürümde de işaretli gelir).
4. **Yayınla** → rapor açılır. `/` adresinde yazılımın sekmesinde görünür.
5. Aynı sürüme yeni koşum gelince listede "yeni koşum" yazar; **Güncelle** ile rapora eklersin.

## 5. Kontrol

- [ ] `test` işinin logunda `yazılım: Kontrol Yazılımı · uygulama sürümü: 1.5.0 (UYGULAMA_SURUMU)`
- [ ] Gizli pencerede `/panel` → giriş sayfası; ekip şifresiyle açılıyor; `/` girişsiz açılıyor
- [ ] Yayınlanan raporda ikinci denemede geçen test "Geçti" görünüyor
- [ ] `HeadlessEnvironmentProbe` gibi testler raporda ve toplamda yok
- [ ] Ayrı koşulan feature'lar aynı raporda
- [ ] Ana sayfada sütun grafiğin altında sürüm numaraları

## 6. Geri dönüş

```bash
sudo systemctl stop divan
sudo cp /opt/divan/divan-0.1.5 /opt/divan/divan
sudo cp /var/lib/divan/tools/ulak-linux-0.1.5 /var/lib/divan/tools/ulak-linux
sudo systemctl start divan
```

Veritabanı 0.1.5 ile de açılır (yeni tablolar kullanılmaz). `.gitlab-ci.yml`'daki `UYGULAMA_ADI`
eski sürümde yok sayılır.
