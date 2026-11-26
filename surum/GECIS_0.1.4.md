# 0.1.3 → 0.1.4 geçiş rehberi

0.1.0'dan geliyorsan önce `surum/GECIS_0.1.3.md`'yi uygula (sunucu, `.env`, jar, test projesi), sonra
buradaki iki adımı yap. 0.1.0'dan doğrudan 0.1.4 ikililerini kurabilirsin; o rehberde "0.1.3" yazan
yerler 0.1.4 için de geçerli.

## 1. Ne değişti

| Değişiklik | Senin için anlamı |
|---|---|
| **Elle Jira bağlama:** senaryo sayfası → Jira → "Var olan issue'ya bağla" (`ABC-123`) | Admin senaryoyu var olan bir issue'ya bağlar. Aynı hatayla kalan senaryolar sonraki gecelerde de kendiliğinden ona bağlanır. "Bağı kaldır" bunu geri alır |
| ⚠ **auto-fix pipeline'ı test edilir:** `auto-fix/...` branch'inde build → test → report kendiliğinden koşar (fix koşmaz) | Eskiden test/report yalnızca `ULAK_FIX_PIPELINE=on` ile koşuyordu. Artık şablondaki `ULAK_FIX_PIPELINE: "senaryo"` ile yalnızca düzeltilen senaryo koşar |

Veritabanı değişikliği yok. Sözleşme aynı (0.1.x): eski ulak yeni divan'la çalışır. Yine de ulak'ı da
değiştir.

## 2. Sunucu

```bash
sudo systemctl stop divan
sudo cp /opt/divan/divan /opt/divan/divan-0.1.3
sudo install -m 755 divan /opt/divan/divan
sudo install -m 755 ulak /var/lib/divan/tools/ulak-linux
sudo install -m 644 ulak.exe /var/lib/divan/tools/ulak-windows.exe   # Windows runner varsa
sudo chown divan:divan /opt/divan/divan /var/lib/divan/tools/*
sudo systemctl start divan
curl -s http://127.0.0.1:8080/api/health          # "surum":"0.1.4"
```

`.env` değişmez.

## 3. Test projesinin `.gitlab-ci.yml`'ı

`surum/linux/ornek.gitlab-ci.yml`'ı kullanıyorsan yenisiyle değiştir (kendi doldurduğun `ULAK_SERVER`,
`ci/settings.xml`, `RUNNER` ve `-D` ayarlarını koru). Elle düzeltmek istersen iki değişiklik var:

1. `test` ve `report` işlerinin `rules:` listesinden şu iki satırı **sil**:
   ```yaml
       - if: '$CI_COMMIT_BRANCH =~ /^auto-fix\// && $ULAK_FIX_PIPELINE != "on"'
         when: never
   ```
   `fix` işindeki `- if: '$CI_COMMIT_BRANCH =~ /^auto-fix\//'` / `when: never` **kalsın**.
2. `variables:` altına ekle:
   ```yaml
     ULAK_FIX_PIPELINE: "senaryo"    # auto-fix pipeline'ında yalnızca düzeltilen senaryo; "hepsi" = bütün e2e
   ```
   GitLab'da (Settings → CI/CD → Variables) `ULAK_FIX_PIPELINE` tanımlıysa o değer geçerli olur. Ya
   `senaryo` yap ya da sil.

⚠ GitLab push seçeneklerini kabul etmezse (fix işinin logunda `push seçeneklerini kabul etmedi`)
auto-fix pipeline'ına filtre gitmez. O durumda artık **bütün e2e** koşar; eskiden hiç koşmuyordu.

## 4. Kontrol

- [ ] Kalan bir senaryonun sayfasında (admin girişliyken) Jira bölümünde "Var olan issue'ya bağla" var.
      `ABC-123` girince "Issue bağlandı.", satırda başlık, durum ve kaynak `elle` görünüyor.
- [ ] Var olmayan anahtar için "Jira'da bu anahtarla issue bulunamadı; bağlanmadı."
- [ ] Ertesi gece aynı hatayla kalan senaryo kendiliğinden o issue'ya bağlı (kaynak `eslesme`).
- [ ] Bir fix MR'ından sonra `auto-fix/...` pipeline'ında build, test ve report koştu, fix işi yok. İş
      logunda `yalnızca bu senaryo koşacak` ya da `-Dcucumber.filter.tags=@YTET_…` görünüyor.

## 5. Geri dönüş

```bash
sudo systemctl stop divan
sudo cp /opt/divan/divan-0.1.3 /opt/divan/divan
sudo systemctl start divan
```

Elle kurulan bağlar eski sürümde de görünür (aynı tablo). `.gitlab-ci.yml`'ı geri almak için silinen
iki satırı geri koy.
