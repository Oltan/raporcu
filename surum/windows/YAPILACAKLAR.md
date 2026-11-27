# Bu sürüme geçiş: yapılacaklar

> **Var olan bir kurulumu güncelliyorsan:** 0.1.0 → 0.1.3 `surum/GECIS_0.1.3.md`, 0.1.3 → 0.1.4 `surum/GECIS_0.1.4.md`, 0.1.4 → 0.1.5 `surum/GECIS_0.1.5.md`, 0.1.5 → 0.2.0 `surum/GECIS_0.2.0.md`; 0.2.0 → 0.2.4 yalnızca ikilileri değiştir (veritabanı, `.env` ve CI aynı).

Derleme: `depo-yapisi` dalı, 0.2.4 (release, x86_64-pc-windows-gnu). Sağlama: `SHA256.txt`.
Veritabanı değişikliği yok; eski veri olduğu gibi kullanılır. Gerçek Windows'ta, GitLab'da ve Jira'da
henüz denenmedi: ilk koşumdan sonra aşağıdaki kontrolleri yapın (§6).

Yollar `docs/KURULUM.md` örneğine göredir (`C:\divan`). Sizde farklıysa kendi yolunuzu kullanın.

## 1. Sunucuyu değiştir

```powershell
# 1) divan'ı durdurun (pencereyi kapatın ya da hizmeti durdurun)
# 2) yedek: veri klasörünün tamamı
Copy-Item -Recurse C:\divan\veri C:\divan\veri-yedek-$(Get-Date -Format yyyyMMdd)
# 3) yeni ikililer
Copy-Item surum\windows\divan.exe C:\divan\divan.exe -Force
Copy-Item surum\windows\ulak.exe  C:\divan\veri\tools\ulak-windows.exe -Force
```

Runner'lar yeni `ulak.exe`'yi sonraki koşumda sunucudan kendisi indirir. Runner makinesinde elle
kopyalanmış bir `ulak.exe` varsa (ör. `preflight` için) onu da değiştirin.

## 2. `.env` (divan) — eklenecek satırlar

```
GEREKSINIM_ONEKI=YTET_
EK_KIMLIK_ONEKLERI=YGÖ-
# JIRA_MATCH_LABEL=divan      # varsayılan zaten divan; label koşulunu kapatmak için off
```

- `.env` **UTF-8** kaydedilmeli (`YGÖ` içinde `Ö` var). Notepad'de *Farklı kaydet → Kodlama: UTF-8*.
- Sonra divan'ı başlatın: `cd C:\divan; .\divan.exe`

## 3. divan arayüzünde

- **Admin → Ayarlar:** bölüm başlığı "Gereksinim (YTET)" olmalı; yer tutucu listesinde `{{YTET}}` ve
  `{{YGÖ}}` görünmeli. Görünmüyorsa `.env` okunmamıştır (§2).
- **Jira bug şablonu → Açıklama:** YGÖ de yazılsın diye ekleyin, ör.:
  ```
  YTET: {{YTET}}
  Bağlı gereksinim: {{YGÖ}}
  ```
  Mevcut `{{doors}}` çalışmaya devam eder (YTET'i yazar); isterseniz `{{YTET}}` ile değiştirin.
  "DOORS numaralarını label olarak ekle" kutusu kaldırıldı; YTET/YGÖ label olmaz.
- Şablonu kaydedin.

## 4. Jira

- divan artık açtığı her bug'a `divan` label'ı ekler ve açık bug ararken yalnızca **Bug türünde ve
  `divan` label'lı** issue'lara bakar (story/task'a fotoğraf gitmez).
- Daha önce açılmış, etiketsiz bir bug'ın eşleşmeye devam etmesini istiyorsanız Jira'da o bug'a
  elle `divan` label'ı ekleyin. Daha önce bağlanmış senaryolar yerel bağla bağlı kalır.

## 5. GitLab ve test projesi

1. **CI değişkeni** (Proje → Settings → CI/CD → Variables):
   | Değişken | Değer | Ne için |
   |---|---|---|
   | `ULAK_FIX_PIPELINE` | `senaryo` | fix push'undan sonra yalnızca düzeltilen senaryo koşar |
   | `ULAK_FIX_VALIDATE_FILTER` | `tags` (isteğe bağlı) | fix doğrulama koşusu tag'le seçilsin (varsayılan: senaryo adı) |
   | `ULAK_CLAUDE_BIN` | ör. `C:\Program Files\Claude\claude.exe` (isteğe bağlı) | Claude Code PATH'te değilse |
2. **`.gitlab-ci.yml`** eski kopyaysa auto-fix kurallarını ekleyin (fix işinin auto-fix branch'inde
   koşmaması için şart):
   ```yaml
   # ulak-test-* ve ulak-report-* işlerinin rules: listesinin EN BAŞINA
       - if: '$CI_COMMIT_BRANCH =~ /^auto-fix\// && $ULAK_FIX_PIPELINE != "on"'
         when: never
   # ulak-fix-* işlerinin rules: listesinin EN BAŞINA
       - if: '$CI_COMMIT_BRANCH =~ /^auto-fix\//'
         when: never
   ```
   `ulak-fix-*` işinin `script` bölümünde `- ulak fix`'ten önce `- ulak degerlendir` yoksa ekleyin.
3. **Feature dosyaları:** gereksinim `@YTET_1234`, ek bağ `@YGÖ-12` (büyük harf, yazıldığı gibi).
   YTET tag'i olmayan senaryolar da çalışır; auto-fix pipeline'ı onları adıyla seçer.
4. **Claude Code kullanıyorsanız:** proje kökünde `.claude/settings.json` ve `CLAUDE.md` olmalı;
   `ulak preflight` eksikse uyarır.
5. **Aktarım betiği:** koşum sayfasındaki düğme artık "YTET CSV"; ilk sütunun başlığı `DOORS ID`
   değil `YTET`, sonda `YGÖ` sütunu var. Betik başlığa bakıyorsa güncelleyin.

## 6. İlk koşumdan sonra kontrol

- [ ] Senaryo sayfasında `YTET_…` ve `YGÖ-…` rozetleri; arama kutusunda `1234` ve `YGÖ-12` bulunuyor.
- [ ] "YTET CSV" doğru sütunlarla iniyor, aktarım betiği okuyor.
- [ ] Açılan Jira bug'ında açıklamada YTET/YGÖ var, label olarak yalnızca `divan` (+ şablondakiler).
- [ ] Bir story ile aynı adı taşıyan kalan senaryo story'ye bağlanmıyor, fotoğraf story'ye gitmiyor.
- [ ] Fix MR'ından sonraki auto-fix pipeline'ında yalnızca o senaryo koştu (iş logunda
      `yalnızca bu senaryo koşacak` ya da `-Dcucumber.filter.tags=@YTET_…`). Fix işi o branch'te koşmadı.
      Fix işinin logunda `push seçeneklerini kabul etmedi` görürseniz GitLab bu özelliği desteklemiyor;
      o durumda test hiç koşmaz, düzeltme yine push edilir.
- [ ] `ULAK_FIX_VALIDATE_FILTER=tags` verdiyseniz doğrulama koşusunda tek senaryo koştu.
- [ ] Ajan zaman aşımına düşerse süreç ağacı kapanıyor (Görev Yöneticisi'nde artık `java`/`node` yok).

Sorun olursa: divan'ı durdurun, eski `divan.exe`'yi ve `veri-yedek-…` klasörünü geri koyun.
