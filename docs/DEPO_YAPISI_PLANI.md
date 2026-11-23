# Depo yapısı, dağıtım ve çok dilli test projeleri — plan

Durum: **taslak, değerlendirme bekliyor** (yanıtlar işlendi, aşağıda). Dal: `depo-yapisi`. Onaylanınca gereksinimler
`GEREKSINIMLER_1.md`'ye SYS-11 olarak eklenir, kararlar `docs/KARARLAR.md`'ye yazılır.

## Amaç

1. divan'ı (sunucu) **değiştirmeden** Java dışındaki test projelerinin (Python, C#) de kullanabilmesi.
2. divan, ulak, dil eklentileri, CI şablonu ve ajan dosyalarının **tek yerden, sürümlü** yönetilmesi.
3. Bir düzeltmenin (ör. sahne ağacı) sahaya **eksiksiz** ulaşması: hangi parçanın hangi sürümde
   olduğu belli olmalı.

## Bugünkü durum

| Parça | Nerede | Sorun |
|---|---|---|
| divan, ulak, ortak tipler | `crates/divan`, `crates/ulak`, `crates/common` | yok; yapı uygun |
| Java destek kodu | `ci/java-sablon/DivanKanit.java`, `ornek-test-projesi/src/test/java/.../CanliOlayEklentisi.java`, `VideoKaydi.java` | kaynak üç yerde; jar'ın kendi modülü yok |
| Java destek jar'ı | `deploy/paket-olustur.ps1` geçici klasörde derliyor, `ci/java-destek/pom.xml` | sürüm sabit `1.0.0`; sahadaki jar'ın hangi kaynaktan olduğu anlaşılmıyor |
| CI şablonu | `ci/gitlab/template/.gitlab-ci.yml` | projeler çoğunlukla **kopyalıyor**; şablon güncellenince eski kopya kalıyor (auto-fix kuralı eksik kopya sahada görüldü) |
| Ajan ve skill şablonları | `ci/opencode/template/`, `ci/claude/template/`, `ci/skills/` | Cucumber + TestFX'e özel |
| Derlenmiş ikililer | `surum/windows/`, `surum/linux/` (git'e commit) | her build'de depo büyüyor; sürümlü indirme yeri yok |
| Depo | GitHub `Oltan/raporcu` | şirket GitLab'ında değil (SW-10.3, tek YAPILACAK) |
| ulak | Maven + Cucumber varsayımlı | Python/C# projesinde `preflight` düşer, test sayımı ve tek test filtresi çalışmaz |

## Kararlar (önerilen)

1. **Tek depo (monorepo).** divan, ulak, eklentiler ve şablonlar aynı sözleşmeyi konuşur (uç adresleri,
   JSON alanları, canlı olay biçimi). Sözleşme değişikliği tek MR'da her tarafı değiştirir; prova
   (`rehearse`) gerçek divan + ulak + örnek projeyle uçtan uca koşar; tek sürüm numarası vardır.
   Ayrı depoya yalnızca bir parçaya ayrı bir ekip sahip olursa geçilir.
2. **Tek ulak, dil profilleri.** Dil başına ayrı program yok: divan ulak'ı tek adresten dağıtır
   (`/api/tools/ulak?os=…`) ve sürüm uyumu tek kuralla denetlenir. Dile özel davranış
   `ULAK_PROFIL` ile seçilir; varsayılan `maven-cucumber` (bugünkü davranış, değişmez).
3. **divan değişmez.** divan yalnızca Allure sonuçlarını ve canlı olay satırlarını okur; her dilin
   Allure adaptörü aynı biçimi üretir. Profil işi tamamen ulak ve eklentilerdedir.
4. **Eklentiler paket olarak dağıtılır.** Test projelerinin içine giren kod (canlı olaylar, hata anı
   kanıtları, video) her dil için kendi paket deposuna sürümlü gider: Maven (Nexus), PyPI (Nexus/aynası),
   NuGet beslemesi. Projeler kaynak kopyalamaz, bağımlılık ekler.
5. **CI şablonu etiketle dahil edilir.** `include: project: 'ekip/divan', ref: vX.Y.Z, file: …`.
   Kopyalama yok; yükseltme `ref` değiştirmektir.
6. **İkililer depoda tutulmaz.** Sürüm etiketinde pipeline'ın ürettiği ikililer GitLab Releases /
   Package Registry'ye gider. `surum/` klasörü geçiş tamamlanınca kaldırılır.
7. **Tek sürüm numarası.** divan, ulak, eklentiler ve şablon aynı etiketi taşır (`v0.3.0`). Uyumluluk
   kuralı bugünkü gibi: ilk iki hane aynıysa uyumlu. Sözleşmeyi bozan değişiklik ikinci haneyi,
   bozmayan üçüncüyü artırır.

## Hedef yapı

```
ekip/divan
├── crates/
│   ├── divan/            sunucu (değişmez)
│   ├── ulak/             profiller: maven-cucumber (varsayılan) · pytest · dotnet
│   ├── common/           sözleşme: API tipleri, sürüm kuralı
│   ├── rehearse/         prova: her profil için ayrı adımlar
│   └── e2esmoke/
├── eklentiler/
│   ├── java/             divan-test-destek: DivanKanit, CanliOlayEklentisi, VideoKaydi (Maven modülü)
│   ├── python/           pytest-divan: canlı olaylar, hata anı kanıtları
│   └── dotnet/           Divan.TestDestek: NUnit/xUnit/Reqnroll dinleyicisi
├── ci/
│   ├── gitlab/template/  maven-cucumber.yml · pytest.yml · dotnet.yml (ortak parçalar tek dosyada)
│   └── ajan/<profil>/    fix ve degerlendir ajanları, skill'ler, opencode.json, CLAUDE.md
├── ornekler/
│   ├── java-javafx/      bugünkü ornek-test-projesi
│   ├── python/
│   └── dotnet/
├── docs/
│   └── SOZLESME.md       uçlar, JSON alanları, canlı olay satırları, sürüm kuralı (eklenti yazanlar için)
└── deploy/               systemd, Windows betikleri, runner-kur.sh
```

## Sözleşme (dilden bağımsız kısım)

Bir dilin desteklenmesi için gereken ve divan'ın beklediği şeyler; `docs/SOZLESME.md`'de tek yerde
yazılır (bugün `docs/ANLIK_AKTARIM.md`, `docs/kod/common/api.md` ve kodda dağınık):

1. **Allure sonuç dosyaları** (`*-result.json`, `*-container.json`, ekler) tek klasörde. Senaryo kimliği
   Allure `historyId`.
2. **Canlı olaylar:** JSON satırları (`senaryo_basladi`, `adim_basladi`, `adim_bitti`, `senaryo_bitti` …)
   tek dosyada, eklenti yazar, ulak gönderir.
3. **Hata anı kanıtları:** ekran görüntüsü, sahne ağacı / DOM / UI ağacı gibi metin ekler (Allure eki).
4. **CI uçları:** `/api/ci/…`, `Authorization: Bearer`, `x-ulak-version`.

## ulak profilleri

| Konu | maven-cucumber (bugün) | pytest | dotnet |
|---|---|---|---|
| Ön kontrol | `pom.xml`, Allure Cucumber adaptörü, paralel kapalı, Monocle yok | `pyproject.toml`/`requirements*.txt`, `allure-pytest`, `pytest-divan` | `*.csproj`, `Allure.NUnit`/`Allure.Xunit`/`Allure.Reqnroll`, `Divan.TestDestek` |
| Varsayılan test komutu | `mvn -B test` + `-D…` | `python -m pytest --alluredir=<sonuç>` | `dotnet test` (sonuç klasörü `allureConfig.json` ile) |
| Beklenen testler | `.feature` dosyaları okunur | `pytest --collect-only -q` | `dotnet test --list-tests` |
| Tek test filtresi (doğrulama, auto-fix) | `-Dcucumber.filter.name=^ad$` / `-Dcucumber.filter.tags` | node id (`dosya::test`) ya da `-k` | `--filter "FullyQualifiedName=…"` |
| Canlı olay eklentisi | `CanliOlayEklentisi` (Cucumber plugin) | `pytest-divan` (pytest hook'ları) | `Divan.TestDestek` (framework dinleyicisi) |
| Sanal ekran (Linux) | Xvfb | masaüstü testinde Xvfb; web testinde gerekmez | aynı |
| Ajan/skill seti | `ci/ajan/maven-cucumber` | `ci/ajan/pytest` | `ci/ajan/dotnet` |
| İzinli yol (`ULAK_FIX_ALLOWED`) | `src/test/` | `tests/` | test projesinin klasörü |

Profil seçimi: `ULAK_PROFIL` CI değişkeni; verilmezse ulak proje kökündeki dosyalardan tahmin eder
(`pom.xml` → maven-cucumber, `pyproject.toml`/`pytest.ini` → pytest, `*.csproj` → dotnet) ve logda yazar.

Ortak kalanlar (profilden bağımsız): koşum açma, `watch`, `upload`/kuyruk, `finish`, sürüm uyumu,
triage, `degerlendir`/`fix` akışı (branch, yol denetimi, push, MR), push seçenekleri.

## Gereksinim önerileri (SYS-11: Dağıtım ve çok dilli test projeleri)

| No | Gereksinim | Doğrulama |
|---|---|---|
| SW-11.1 | Depo şirket GitLab'ında; divan deposunun kendi pipeline'ı: test, clippy, prova, Linux + Windows derlemesi, eklenti testleri | pipeline yeşil |
| SW-11.2 | Sürüm etiketi ikilileri (Linux, Windows) ve eklenti paketlerini yayınlar; ikililer depoda tutulmaz | etiket pipeline'ı, Release sayfası |
| SW-11.3 | Java destek kodu `eklentiler/java` Maven modülünde, sürümü depo etiketiyle aynı; örnek proje onu bağımlılık olarak kullanır | `test_sw_11_3_*`, prova |
| SW-11.4 | CI şablonu `include … ref: <etiket>` ile kullanılır; kopya gerekmez | örnek projede include |
| SW-11.5 | `docs/SOZLESME.md`: uçlar, JSON alanları, canlı olay satırları, ekler, sürüm kuralı | belge + sözleşme testleri |
| SW-11.6 | ulak profil altyapısı; `maven-cucumber` bugünkü davranışla birebir (gerileme yok) | mevcut testler + prova değişmeden geçer |
| SW-11.7 | `pytest` profili: ön kontrol, komut, test sayımı, tek test filtresi | `test_sw_11_7_*`, `ornekler/python` ile prova |
| SW-11.8 | `pytest-divan` eklentisi: canlı olaylar ve hata anı kanıtı | eklenti testleri + prova |
| SW-11.9 | `dotnet` profili ve `Divan.TestDestek` | `test_sw_11_9_*`, `ornekler/dotnet` |
| SW-11.10 | Profil başına ajan/skill seti; `degerlendir` ve `fix` istemleri profile göre | prova (sahte ajan) |
| SW-11.11 | Profil otomatik tahmini ve logda gösterimi | `test_sw_11_11_*` |

## Geçiş aşamaları

Her aşama ayrı MR; her aşamadan sonra sistem bugünkü gibi çalışmaya devam eder.

| Aşama | İş | Bağımlılık |
|---|---|---|
| 0 | Depoyu şirket GitLab'ına taşı (`ekip/divan`); GitHub salt okunur arşiv | — |
| 1 | divan deposunun pipeline'ı (SW-11.1). Runner: Linux, Rust + mingw + musl + JDK + Maven (Windows testleri Wine ile ya da ayrı Windows runner) | 0 |
| 2 | `eklentiler/java` Maven modülü; `DivanKanit`, `CanliOlayEklentisi`, `VideoKaydi` tek kaynak; örnek proje bağımlılık olarak kullanır; jar sürümü = depo etiketi (SW-11.3) | 1 |
| 3 | Sürüm pipeline'ı: etiket → Release (ikililer) + Nexus (jar); `surum/` kaldırılır; kurulum belgeleri indirme adresine göre güncellenir (SW-11.2) | 1, 2 |
| 4 | Projeler `include … ref:` kullanır; kopyalanmış eski şablonlar değiştirilir (SW-11.4) | 3 |
| 5 | `docs/SOZLESME.md` (SW-11.5) | — (her an) |
| 6 | ulak profil altyapısı; yalnızca `maven-cucumber` (SW-11.6, 11.11). Davranış değişmez | 5 |
| 7 | İlk yeni profil (Python ya da C#, hangisi önce gerekiyorsa) + eklentisi + örnek proje + prova (SW-11.7–11.10) | 6 |

Aşama 2 tek başına bile değerli: sahne ağacı düzeltmesi gibi Java tarafı değişikliklerinin sahaya
ulaşıp ulaşmadığı jar sürümünden görülür.

## Kullanıcının yanıtları (2026-11-23)

| Soru | Yanıt | Plana etkisi |
|---|---|---|
| Paket deposu | Şirkette **Nexus var** | Java jar'ı (ve ileride PyPI/NuGet paketleri) Nexus'a; GitLab Package Registry gerekmez |
| İlk dil | **Önce Java** tamamlanacak | Aşama 6–7 (yeni profiller) bekler; Java tarafı eksikleri öne alınır (aşağıda "Java için sıradakiler") |
| divan deposunun runner'ı | **Windows runner kurulabilir** | Aşama 1'de Linux runner (derleme, test, prova) + Windows runner (Windows testleri, Windows provası) |
| Proje sayısı | Şu an **tek proje** | Eski kopya geçişi (aşama 4) tek projede yapılır |
| Sürüm etiketi | Etiket **deponun tamamı** içindir (divan + ulak + eklentiler + şablon) | İlk etiket: `v0.1.0` → `33c04cb` (sahadaki ikililerle aynı kod) |
| Ayrı ekip | Şu an yok | Tek depo kararı kesinleşir |

## Java için sıradakiler (yeni profillerden önce)

| Öncelik | İş | Neden |
|---|---|---|
| 1 ✅ | `eklentiler/java` Maven modülü (1.1.0, MANIFEST'te sürüm + commit); **Nexus yok**, jar `surum/java/`'dan elle dağıtılır (kullanıcı kararı) | sahadaki jar'ın hangi kaynaktan olduğu bilinmiyor; sahne ağacı düzeltmesi buna takıldı |
| 2 | Senaryo kimliği satır kaymasına dayanıklı (Allure `historyId` = dosya yolu + satır; `testCaseId` = yol + ad) | feature dosyasına satır eklemek geçmişi, flaky tespitini ve açık MR takibini koparıyor |
| 3 ✅ | Zaman aşımında senaryo atlanır, art arda `ULAK_ART_ARDA_ZAMAN_ASIMI` (3) aşımda durulur; logda ajanın çıktı verip vermediği (kullanıcı kararı: atla) | gece işi boşuna 45 dk'ya kadar bekleyebiliyor; yavaş model değerlendirmeyi toptan kesiyor |
| 4 ✅ | ulak işin başında kullandığı sınırları ve kaynaklarını yazsın (`ULAK_FIX_MAX=… (CI değişkeni / varsayılan)`), sayı olmayan değerde uyarsın | "değişkeni görmedi mi?" sorusu logdan cevaplansın |
| 5 ✗ | `fx:id` taraması (preflight uyarısı) — şimdilik yapılmıyor (kullanıcı kararı) | ajanın güvenilir seçici yazamadığı kontroller önceden görünsün |
| 6 ✗ | Sahne ağacı için Java birim testi — kullanıcı kendi test projesine ekledi | düzeltme ileride sessizce bozulmasın |
| 7 | Koşumu başlatan kullanıcı ve runner adı raporda (`GITLAB_USER_LOGIN`, `CI_RUNNER_DESCRIPTION`) | birden çok VM'de "kimin koşumu, hangi makine" |

## Açık sorular

1. **Paket deposu:** Şirkette Nexus/Artifactory var mı (Maven, PyPI, NuGet)? Yoksa GitLab Package
   Registry'nin her üçü de kullanılabilir.
2. **İlk yeni dil:** Python mu C# mı? Test çerçevesi (pytest, NUnit, xUnit, Reqnroll) ve test türü
   (masaüstü arayüz mü, web mi, API mi)? Masaüstü değilse Xvfb ve video gerekmez, eklenti küçülür.
3. **divan deposunun runner'ı:** Windows ikilisi Linux'ta mingw ile derleniyor; Windows testleri için
   Wine mı, ayrı Windows runner mı?
4. **Eski kopyalar:** Kopyalanmış `.gitlab-ci.yml` kullanan projelerin listesi ve geçiş tarihi.
5. **Sürüm sıklığı:** Her MR sonrası mı, belirli aralıklarla mı etiket atılacak?
6. **Ayrı ekip sahipliği:** Bir eklentiyi başka bir ekip geliştirecekse o parça ayrı depoya alınabilir;
   bu durumda sözleşme belgesi ve sözleşme testleri şart.

## Riskler

- **Profil altyapısında gerileme:** Aşama 6 yalnızca yeniden düzenleme olmalı; mevcut 150+ test ve
  60 prova adımı değişmeden geçmeden yeni profil eklenmez.
- **Tek sürüm numarası:** Yalnızca Python eklentisi değişse de bütün parçalar yeni etiket alır. Basitlik
  için kabul edilir; gerekirse eklentiler kendi üçüncü hanesini artırabilir.
- **Allure adaptör farkları:** pytest ve .NET adaptörlerinin `historyId`, `fullName`, durum (`broken` /
  `failed`) davranışı Cucumber'dan farklı olabilir. Her profil, divan'ın gerçek çıktıyla test edilmesi
  için `testdata/allure/<profil>/` altında gerçek örnekler getirmeli (CLAUDE.md kuralı).
