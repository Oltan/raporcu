# Evde deneme (GitLab olmadan)

Tüm akışı tek komutla, gerçek JavaFX + Cucumber + TestFX testleriyle koşturur:
raporcu sunucusu açılır → `stajci preflight` → `stajci start` (testler koşar, canlı ekran
tarayıcıda açılır) → `stajci upload` → `stajci finish` → dashboard.

Test projesi olarak `ornek-test-projesi/` kullanılır: giriş ekranı olan küçük bir JavaFX
uygulaması ve 7 senaryo. Bilerek bir kalan (assertion), bir broken (olmayan düğüm), bir flaky
(ilk denemede kalır, rerun'da geçer) ve bir tanımsız adım içerir. Böylece raporcu'nun her durumu
görülür. Proje bir kopyası üzerinde koşar, asıl klasör kirlenmez.

## Gerekenler

| | Linux (Ubuntu) | Windows |
|-|----------------|---------|
| Java 21 | `sudo apt install openjdk-21-jdk` | Temurin 21, PATH'te `java` |
| Maven | `sudo apt install maven` | Maven zip'i açılır, `bin` PATH'e eklenir |
| Ekran | `sudo apt install xvfb xauth` (testler sanal ekranda koşar) | kendi masaüstün (pencereler açılır) |
| Video (isteğe bağlı) | `sudo apt install ffmpeg` | ffmpeg, PATH'te |
| İnternet | yalnızca ilk koşumda (Maven bağımlılıkları, ~100 MB) | aynı |

## Çalıştırma

Paketin kökünde (raporcu ikilisinin yanında):

```bash
./ev-denemesi.sh        # Linux, 1 koşum
./ev-denemesi.sh 3      # 3 koşum: dashboard'da gidişat çizgisi ve flaky tespiti görünür
```

Windows: `ev-denemesi.cmd` dosyasına çift tıklayın ya da PowerShell'de
`powershell -ExecutionPolicy Bypass -File ev-denemesi.ps1 -Kosu 3`.
**Windows'ta testler gerçek pencere açar ve fareyi kullanır; koşum sırasında fareye ve klavyeye
dokunmayın.** Bir koşum yaklaşık 30 saniye sürer (ilk koşumda Maven indirmesi ayrıca).

Port 8080 doluysa: `PORT=8090 ./ev-denemesi.sh` / `ev-denemesi.ps1 -Port 8090`.

## Neye bakmalı

1. **Canlı ekran** (`/runs/<n>/live`, kendiliğinden açılır): şu an koşan senaryo, adımları,
   geçen süre; biten senaryolar ve kalanların ilk hata satırı, ekran görüntüsü küçük resmi.
2. **Koşum raporu** (`/runs/<n>`): durum dağılımı, feature tablosu, feature'a tıklayınca
   senaryolar, senaryoya tıklayınca adımlar, hata, ekran görüntüsü, **sahne ağacı** ve **video**.
3. **"Yavaş açılan karşılama mesajı"**: 2 deneme; ilki kaldı, ikincisi geçti (rerun).
4. **Dashboard** (`/`): son koşum donut grafiği; birden çok koşumda gidişat çizgisi ve flaky listesi.
5. **Arama** (`/search`): ör. DOORS numarası `DOORS-1003` ya da `@sample-fail` tag'i; CSV dışa aktarma.
6. Admin işleri (paylaşım linki, triage, Jira düğmesi): betiği kapatın, `ev-denemesi-veri`
   klasöründe `raporcu set-admin-password` (Windows: `..\raporcu.exe set-admin-password`)
   çalıştırın; şifre o klasördeki `.env`'e yazılır. Betiği yeniden başlatınca `/login` ile girin.
   Önceki koşumlar silinmez.

Her şey `ev-denemesi-veri/` klasöründe; silmek yeterli.

## Kendi projenle denemek

`ornek-test-projesi` yerine kendi projeni koymak için: önce `java/` klasöründeki dosyaları
(Hooks, VideoKaydi, CanliOlayEklentisi, junit-platform.properties, features.txt) projene ekle
(`docs/YENI_PROJE_EKLEME.md`), sonra projeyi `ornek-test-projesi` adıyla betiğin yanına koy ya da
betikteki `PROJE_KAYNAK` satırını değiştir. `stajci preflight` eksik bir şey varsa açıkça söyler.
