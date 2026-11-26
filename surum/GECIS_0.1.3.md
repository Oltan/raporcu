# 0.1.0 → 0.1.3 geçiş rehberi

Sahada **0.1.0** (ya da 0.1.1 / 0.1.2) çalışan bir kurulumu 0.1.3'e geçirir. Adımlar Linux sunucu
(systemd hizmeti) içindir; dosyalar `surum/linux/`, Windows runner kullanıyorsan `surum/windows/ulak.exe`,
Java eklentisi `surum/java/`. Hepsi SSH'den yapılır.

Şu an hangi sürüm:

```bash
curl -s http://127.0.0.1:8080/api/health      # {"durum":"ok","surum":"0.1.0"}
/var/lib/divan/tools/ulak-linux --version     # ulak 0.1.0
```

## 1. Ne değişti

| Sürüm | Değişiklik | Senin için anlamı |
|---|---|---|
| 0.1.1 | Zaman aşımına düşen senaryo **atlanır**, sıradakine geçilir; art arda 3 aşımda durulur (`ULAK_ART_ARDA_ZAMAN_ASIMI`) | Eskiden ilk zaman aşımında bütün değerlendirme duruyordu |
| 0.1.1 | İşin başında `ayarlar:` satırı (sınırlar ve nereden geldikleri) | Değişkenin okunup okunmadığı logdan görülür |
| 0.1.1 | Fix'te zaman aşımındaki yarım değişiklik geri alınır | Yarım düzeltme MR'a gitmez |
| 0.1.1 | Java jar **1.1.0** (sahne ağacı sekme/kaydırma içine iniyor; MANIFEST'te sürüm) | Jar'ı değiştirmen gerekir (§5) |
| 0.1.2 | Feature dosyası olmayan sonuçlar (JUnit testleri) değerlendirme ve fix dışı | Raporda kalırlar, ajan uğraşmaz |
| 0.1.3 | **Uygulama sürümü raporda** (`UYGULAMA_SURUMU`) | Başlıkta "Uygulama 1.5.0", arama ve CSV süzgeci |
| 0.1.3 | **`@ABC-123` Jira tag'leri**: başlık/durum Jira'dan çekilir | ⚠ `JIRA_PROJECT` doluysa o projenin `@<PROJE>-123` tag'leri artık Jira issue'su sayılır ve kalan senaryo o issue'ya bağlanır |
| 0.1.3 | ⚠ `ULAK_DEGERLENDIR_MAX` / `ULAK_FIX_MAX` **varsayılanı sınırsız** (eskiden 20 / 3) | Gece işi uzar; fix işine `timeout` gerekir (§6) |
| 0.1.3 | Değişken adı toleranslı (`ulak-fix-max` de okunur, uyarıyla); işin başında `özet:` satırı | |
| CI | ⚠ Push'ta test koşmaz; yalnızca schedule, Run pipeline, divan düğmesi, auto-fix | `main`'e merge test başlatmaz; gece schedule'ı test eder |
| CI | Senaryo seçimi Run pipeline formunda (`ULAK_TAGS`), `features.txt` gereksiz | Commit'siz yönetim |
| CI | Merkezi divan (`ULAK_SERVER`), VM seçimi (`RUNNER`), fix yalnızca `ajan` etiketli VM'de | Birden çok VM |

Veritabanı: iki yeni migration (`0007_uygulama_surumu`, `0008_jira_issue`) açılışta kendiliğinden uygulanır.
divan bundan önce kendisi yedek **almaz**; yedeği §2'de sen alırsın. Sunucu ile ulak arasındaki sözleşme
uyumlu kaldı (0.1.x).

## 2. Hazırlık

```bash
sudo systemctl stop divan
sudo cp -a /var/lib/divan /var/lib/divan-yedek-$(date +%Y%m%d)     # veri yedeği
sudo cp /opt/divan/divan /opt/divan/divan-0.1.0                     # eski ikili (geri dönüş için)
sudo cp /var/lib/divan/tools/ulak-linux /var/lib/divan/tools/ulak-linux-0.1.0
```

- Test projesinin şu anki `.gitlab-ci.yml`'ının bir kopyasını al.
- Bir gece koşusu çalışmıyor olmalı (GitLab → Build → Pipelines).

## 3. Sunucu

`surum/linux/` altındaki `divan` ve `ulak` makinedeyken (ör. `scp` ile `/tmp/divan-kurulum/`):

```bash
cd /tmp/divan-kurulum
sudo install -m 755 divan /opt/divan/divan
sudo install -m 755 ulak /var/lib/divan/tools/ulak-linux
sudo install -m 644 ulak.exe /var/lib/divan/tools/ulak-windows.exe   # Windows runner varsa
sudo chown divan:divan /opt/divan/divan /var/lib/divan/tools/*
```

## 4. `.env` (`/opt/divan/.env`)

Yeni, isteğe bağlı:

```
# @ABC-123 tag'i Jira issue'su sayılacak projeler (virgülle). Verilmezse JIRA_PROJECT.
JIRA_TAG_PROJELERI=STAJ
```

0.1.0'da gelmiş olanlar; yoksa ekle:

```
GEREKSINIM_ONEKI=YTET_
EK_KIMLIK_ONEKLERI=YGÖ-
# JIRA_MATCH_LABEL=divan    # varsayılan zaten divan
```

Dosya UTF-8 kalmalı (`YGÖ`). Sonra:

```bash
sudo systemctl start divan
curl -s http://127.0.0.1:8080/api/health          # "surum":"0.1.3"
journalctl -u divan -n 100 | grep "migration uygulandı"   # 0007_uygulama_surumu, 0008_jira_issue
```

## 5. Java eklentisi (jar 1.1.0)

0.1.0'daki jar `1.0.0` ise değiştir: `surum/java/README.md` (Nexus'suz iki yol; birden çok VM için
jar'ı test reposunun içinde tutmak önerilir). Test projesinin `pom.xml`'inde bağımlılık sürümü `1.1.0`.
Kontrol: `unzip -p divan-test-destek-1.1.0.jar META-INF/MANIFEST.MF` → `Implementation-Version: 1.1.0`.

## 6. Test projesi

1. **`pom.xml`:** test edilen uygulamanın sürümü özellik olsun:
   ```xml
   <properties>
     <uygulama.version>1.4.0</uygulama.version>   <!-- varsayılan -->
   </properties>
   ...
   <dependency>
     <groupId>com.sirket</groupId>
     <artifactId>uygulama</artifactId>
     <version>${uygulama.version}</version>
   </dependency>
   ```
2. **`features.txt`:** senaryo seçimini GitLab'dan yöneteceksen sil. Durduğu sürece boş `ULAK_TAGS`
   (schedule dahil) bu dosyayla sınırlanır.
3. **`.gitlab-ci.yml`:** `surum/linux/ornek.gitlab-ci.yml` ile değiştir; doldurulacaklar:
   - `ULAK_SERVER`: merkezi divan'ın adresi (divan aynı makinedeyse `http://127.0.0.1:8080`)
   - `ci/settings.xml` yolu (build ve `ULAK_TEST_COMMAND`)
   - `ULAK_TEST_COMMAND`'daki kendi `-D` ayarların; JUnit testleri rapora girmesin istersen yorumdaki
     `-Dtest=…` satırı
   - `RUNNER` varsayılanı (`linux`) ve fix işinin etiketi (`ajan`) runner'larındaki etiketlerle aynı olmalı;
     tek VM varsa ona hem `linux` hem `ajan` etiketi ver
4. Commit et; push artık pipeline başlatmaz (`workflow` kuralı).

## 7. GitLab ayarları

| Yer | Ne |
|---|---|
| Settings → CI/CD → Variables | `CI_API_TOKEN` (masked, protected **değil**); self-healing için `VLLM_BASE_URL`, `VLLM_API_KEY`, `VLLM_MODEL`, `ULAK_FIX_TOKEN`; istersen `ULAK_FIX_PIPELINE=senaryo`. `ULAK_FIX_MAX` / `ULAK_DEGERLENDIR_MAX` yalnızca sınır istiyorsan; ad tam olarak böyle (büyük harf, alt çizgi) |
| Build → Pipeline schedules | Gece schedule'ı: değişken vermezsen hepsi koşar. Belirli uygulama sürümü: `UYGULAMA_SURUMU=1.5.0`; belirli VM: `RUNNER=<etiket>`; alt küme: `ULAK_TAGS=@smoke` |
| Runner sayfası | Etiketler (`linux`, kişi adı, `ajan`); **Maximum job timeout ≥ 8 saat** (fix işi `timeout: 8h`) |
| Settings → Webhooks | Merge request webhook adresi `http://<divan>:8080/api/gitlab/webhook` (adres değiştiyse) |

Elle koşum: Build → Pipelines → **Run pipeline** → formda `ULAK_TAGS` ve `UYGULAMA_SURUMU` kutuları
açıklamalarıyla çıkar; boş bırakmak = hepsi / pom'daki sürüm.

## 8. İlk koşum kontrol listesi

Elle bir pipeline başlat (`UYGULAMA_SURUMU` doldurarak) ve bak:

- [ ] test işi: `koşum açıldı: N senaryo bekleniyor`, `testler başlıyor: xvfb-run … -Duygulama.version=…`
- [ ] koşum sayfası başlığında **Uygulama 1.5.0**; dashboard listesinde `uygulama 1.5.0`
- [ ] fix işi: `ayarlar: ULAK_DEGERLENDIR_MAX=sınırsız (varsayılan) …` ve iki `özet:` satırı
      (kaç senaryo değerlendirilecek / düzeltilecek, hedefe göre atlananlar)
- [ ] zaman aşımı olursa `UYARI: zaman aşımı (… sn), atlandı: …` ve iş devam ediyor
- [ ] `@ABC-123` tag'li senaryoda rozet `ABC-123 · <durum>`, üzerine gelince başlık (koşum bittikten
      sonra birkaç saniye içinde; Jira bağlı olmalı)
- [ ] `main`'e bir push yap: pipeline **başlamamalı**
- [ ] ertesi sabah gece schedule'ı hepsini koşmuş olmalı

## 9. Geri dönüş

```bash
sudo systemctl stop divan
sudo cp /opt/divan/divan-0.1.0 /opt/divan/divan
sudo cp /var/lib/divan/tools/ulak-linux-0.1.0 /var/lib/divan/tools/ulak-linux
sudo systemctl start divan
```

Veritabanı eski sürümle de açılır (yeni sütun ve tablo kullanılmaz, eski veriler yerinde). Tam eski hâle
dönmek için `/var/lib/divan-yedek-…` klasörünü geri kopyala. Test projesinde eski `.gitlab-ci.yml`'ı ve
`pom.xml`'i geri koy.
