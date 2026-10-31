# Java test projesini divan + ulak'a bağlama (LLM rehberi)

Bu rehber, mevcut bir Java test projesini divan'a bağlar. Küçük bir yerel model (ör.
`qwen3.8-flash-next`) ve yavaş bir bilgisayar için yazıldı: **model tahmin etmez, makine doğrular.**

- **divan:** Sonuçları gösteren sunucu.
- **ulak:** CI'da testleri koşan ve sonuçları divan'a gönderen araç.
- Hedef: `ulak preflight` hatasız geçer, bir pipeline'ın sonuçları divan'da görünür.

## İnsan için: nasıl kullanılır

1. Modele aşağıdaki **Sistem istemi**ni ver.
2. Her adım için **yeni bir sohbet** aç. Yalnızca o adımın bölümünü ve adımın "Modele ver"
   satırındaki dosyaları yapıştır. Tüm repoyu ya da bu belgenin tamamını verme.
3. Komutları kendin çalıştır, çıktının yalnızca son 30 satırını modele ver.
4. Adım geçince commit at. Sonraki adıma ancak **Kontrol** geçtikten sonra başla.
5. Aynı hata iki denemede çözülmezse dur, modeli zorlama. **Hata tablosu**na bak.

Yavaş bilgisayar için:

- Model yanıt üretirken test koşma, ikisi aynı belleği paylaşır.
- Önce `ulak preflight` kullan: JVM açmaz, saniyeler sürer.
- Bağımlılıklar inince `mvn -o` ile çevrimdışı koş, tüm testler yerine tek senaryo koş.
- Yerelde `ULAK_VIDEO=false` ile video kaydını kapat.

### Sistem istemi

```text
Sen bir Java derleme yapılandırma yardımcısısın.
Kurallar:
1. Yalnızca sana verilen dosyaları değiştir. Görmediğin dosya hakkında tahmin yürütme.
2. Testin içeriğini, assertion'ları ve src/main altını değiştirme.
3. Uzun Java dosyası yazma. Komutla kopyalanan dosyaları yeniden üretme.
4. Sürüm numarası uydurma. Yalnızca rehberdeki ya da dosyadaki sürümleri kullan.
5. Yanıtın yalnızca istenen değişiklik olsun: dosya yolu ve değişen bölüm.
6. Bilgi eksikse tek bir soru sor ve dur.
```

## 0. Uygunluk kontrolü

Modele gerek yok. Proje kökünde şu dört soruyu yanıtla:

| Soru | Nasıl bakılır | "Hayır" ise |
|------|---------------|-------------|
| Maven mı? | Kökte `pom.xml` var mı | Gradle desteklenmiyor: `ulak preflight` `pom.xml` ister. Önce Maven'a geçir |
| Cucumber 7 + JUnit Platform mı? | `pom.xml` içinde `cucumber-junit-platform-engine` | Düz JUnit testlerinde `.feature` yok; divan'a bağlanamaz. Önce senaryoları Cucumber'a taşı |
| TestFX ile JavaFX mi? | `pom.xml` içinde `testfx-core` | Sonuçlar yine görünür, ama 3. ve 4. adımlardaki kanıt kodu çalışmaz. O adımları atla, projeye özel yaz |
| Java 17 veya üstü mü? | `java -version` | Şablon Java dosyaları derlenmez. `docs/CANLI_EKRAN_KURULUM.md` sorun giderme bölümüne bak |

Şunlar da olmamalı: `openjfx-monocle`, `testfx.headless=true`, `prism.order=sw`,
`cucumber.execution.parallel.enabled=true`. Varsa kaldır: testler gerçek ekranda, sırayla koşar.

## Değişkenler

Komutlardan önce kendi değerlerinle ayarla. `PAKET`, adım tanımlarının bulunduğu Java paketidir.

```bash
DIVAN=$HOME/divan              # divan deposu
PAKET=com.sirket.test          # adım tanımlarının paketi
SUNUCU=http://127.0.0.1:8080   # divan adresi
```

```powershell
$DIVAN = "C:\Users\ol_ta\projects\raporcu"
$PAKET = "com.sirket.test"
$SUNUCU = "http://127.0.0.1:8080"
```

## Adım 1: `pom.xml`

**Modele ver:** `pom.xml`, bu adım.

Şunları ekle, var olanları tekrar ekleme:

```xml
<!-- <properties> içine -->
<allure.version>2.35.5</allure.version>
<test.xmx>768m</test.xmx>
<rerun.count>1</rerun.count>
```

```xml
<!-- <dependencyManagement><dependencies> içine -->
<dependency>
    <groupId>io.qameta.allure</groupId>
    <artifactId>allure-bom</artifactId>
    <version>${allure.version}</version>
    <type>pom</type>
    <scope>import</scope>
</dependency>
```

```xml
<!-- <dependencies> içine -->
<dependency>
    <groupId>io.qameta.allure</groupId>
    <artifactId>allure-cucumber7-jvm</artifactId>
    <scope>test</scope>
</dependency>
<dependency>
    <groupId>org.openjfx</groupId>
    <artifactId>javafx-swing</artifactId>
    <version>${javafx.version}</version>
    <scope>test</scope>
</dependency>
```

`javafx.version` özelliği yoksa projedeki `javafx-controls` sürümünü yaz.

```xml
<!-- <build><plugins> içine; bu eklentiler varsa yalnızca eksik ayarları ekle -->
<plugin>
    <groupId>org.apache.maven.plugins</groupId>
    <artifactId>maven-compiler-plugin</artifactId>
    <version>3.15.0</version>
</plugin>
<plugin>
    <groupId>org.apache.maven.plugins</groupId>
    <artifactId>maven-surefire-plugin</artifactId>
    <version>3.5.4</version>
    <configuration>
        <forkCount>1</forkCount>
        <reuseForks>true</reuseForks>
        <argLine>-Xmx${test.xmx} -Djava.awt.headless=false -Dfile.encoding=UTF-8</argLine>
        <rerunFailingTestsCount>${rerun.count}</rerunFailingTestsCount>
        <systemPropertyVariables>
            <allure.results.directory>${project.build.directory}/allure-results</allure.results.directory>
        </systemPropertyVariables>
    </configuration>
</plugin>
```

Projede `argLine` zaten varsa üçünü de koruyarak birleştir, silme.

**Kontrol:** `mvn -B -q test-compile` hatasız biter.

## Adım 2: Cucumber ayarları

**Modele ver:** `src/test/resources/junit-platform.properties`, bu adım.

`cucumber.plugin` satırı şöyle olmalı; `pretty` dışında başka eklenti varsa koru.
`com.sirket.test` yerine `PAKET` değerini yaz:

```properties
cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm, com.sirket.test.CanliOlayEklentisi
cucumber.execution.parallel.enabled=false
```

`src/test/resources/allure.properties` dosyasını oluştur:

```properties
allure.results.directory=target/allure-results
```

Eklentiler runner sınıfında `@ConfigurationParameter` ile veriliyorsa aynı adları oraya ekle.

**Kontrol:** 3. adımdan sonra yapılır; sınıf henüz yok.

## Adım 3: Hazır Java dosyalarını kopyala

**Model gerekmez.** Bu dosyaları modele yazdırma, komutla kopyala.

```bash
HEDEF=src/test/java/$(echo "$PAKET" | tr . /)
mkdir -p "$HEDEF"
for f in ornek-test-projesi/src/test/java/com/ornek/test/CanliOlayEklentisi.java \
         ornek-test-projesi/src/test/java/com/ornek/test/VideoKaydi.java \
         ci/java-sablon/DivanKanit.java; do
  sed "s/com\.ornek\.test/$PAKET/g" "$DIVAN/$f" > "$HEDEF/$(basename "$f")"
done
```

```powershell
$HEDEF = "src\test\java\" + ($PAKET -replace '\.', '\')
New-Item -ItemType Directory -Force $HEDEF | Out-Null
foreach ($f in "ornek-test-projesi\src\test\java\com\ornek\test\CanliOlayEklentisi.java",
               "ornek-test-projesi\src\test\java\com\ornek\test\VideoKaydi.java",
               "ci\java-sablon\DivanKanit.java") {
  $metin = [IO.File]::ReadAllText("$DIVAN\$f").Replace("com.ornek.test", $PAKET)
  [IO.File]::WriteAllText("$PWD\$HEDEF\$(Split-Path $f -Leaf)", $metin)
}
```

**Kontrol:** `mvn -B -q test-compile` hatasız biter.

## Adım 4: Hook'a kanıt ekle

**Modele ver:** `@Before` ve `@After` içeren sınıf (genelde `Hooks.java`), bu adım.

Mevcut hook'u koru, yalnızca şu satırları ekle. `stage`, `FxToolkit.registerPrimaryStage()`
sonucudur; sınıfta yoksa alan olarak ekle.

```java
// sınıf alanları
private VideoKaydi video;

// @Before içinde, uygulamayı açmadan ÖNCE
video = VideoKaydi.baslat();

// @After içinde, pencereyi kapatmadan ÖNCE. Metoda Scenario senaryo parametresi ekle.
DivanKanit.ekle(senaryo, stage);
if (video != null) {
    byte[] kayit = video.durdur(senaryo.isFailed());
    if (kayit != null) {
        senaryo.attach(kayit, "video/mp4", "Video");
    }
}
```

`import io.cucumber.java.Scenario;` satırını ekle. Pencere kapatma satırlarının yerini değiştirme.

**Kontrol:** `mvn -B -q test-compile` hatasız biter.

## Adım 5: Koşulacak senaryolar ve `ulak` kontrolü

**Modele ver:** Hiçbir şey. `.feature` dosyalarına bakıp projede geçen bir tag seç.

Proje kökünde `features.txt` oluştur, her satıra bir tag ifadesi yaz:

```text
@smoke
```

Satırlar `or` ile birleşir. `@` ile başlamayan satır geçersizdir.

`.gitignore` dosyasına ekle:

```text
target/
.ulak/
ulak.env
ulak.exe
ulak
```

`ulak`'ı divan'dan indir ve ön kontrolü koş. divan çalışıyor olmalı.

```bash
curl -fsS -o ulak "$SUNUCU/api/tools/ulak?os=linux" && chmod +x ulak && ./ulak preflight
```

```powershell
curl.exe -fsS -o ulak.exe "$SUNUCU/api/tools/ulak?os=windows"; .\ulak.exe preflight
```

**Kontrol:** Son satır `ön kontrol geçti`, `HATA` yok. `UYARI` satırlarını oku ve kapat.

## Adım 6: Tek senaryoyu yerelde koş

**Model gerekmez.** `AD` yerine geçen bir senaryonun tam adını yaz.

```bash
ULAK_VIDEO=false xvfb-run -a mvn -B test -Dcucumber.filter.name="^AD$"
```

```powershell
$env:ULAK_VIDEO = "false"; mvn -B test "-Dcucumber.filter.name=^AD$"
```

Windows'ta pencere açılır, koşum bitene kadar fareye dokunma.

**Kontrol:**

```bash
ls target/allure-results/*-result.json | wc -l
wc -l target/ulak-canli/olaylar.jsonl
```

```powershell
(Get-ChildItem target\allure-results\*-result.json).Count
(Get-Content target\ulak-canli\olaylar.jsonl).Count
```

İkisi de 0'dan büyük olmalı.

## Adım 7: CI

**Modele ver:** Hiçbir şey. Şablonu kopyala.

```bash
cp "$DIVAN/ci/gitlab/template/.gitlab-ci.yml" .gitlab-ci.yml
```

```powershell
Copy-Item "$DIVAN\ci\gitlab\template\.gitlab-ci.yml" .gitlab-ci.yml
```

`.gitlab-ci.yml` içinde iki satırı değiştir:

- `ULAK_OS: "linux"`: Runner Windows ise `"windows"` yap.
- `# ULAK_FIX: "off"`: Baştaki `#` işaretini kaldır. Self-healing kurulana kadar kapalı kalsın.

GitLab'da **Settings → CI/CD → Variables** altına ekle: `CI_API_TOKEN`, Masked, Protect kapalı.
Değer, divan `.env` dosyasındaki ile aynı olmalı. Token'ı modele verme, koda yazma.

Runner'ın etiketi `linux` ya da `windows` olmalı. Windows'ta runner Windows hizmeti olarak değil,
masaüstü oturumunda çalışmalı ve `mvn` onun PATH'inde olmalı.

**Kontrol:** Push sonrası divan'da **Canlı** sayfası koşumu gösterir, koşum `finished` olur.

## Hata tablosu

Mesajı aynen ara. Bulduğun satırı modele ver, tüm logu verme.

| Görülen mesaj | Sebep | Çözüm |
|---------------|-------|-------|
| `Allure adaptörü eksik` | Bağımlılık yok | Adım 1 |
| `Cucumber Allure eklentisi ayarlı değil` | `cucumber.plugin` yanlış ya da başka dosyada | Adım 2 |
| `Canlı olay eklentisi` uyarısı | Sınıf ya da ayar eksik | Adım 2 ve 3 |
| `features.txt içindeki @x tag'i hiçbir senaryoda yok` | Yanlış tag | Adım 5 |
| `Headless/Monocle ayarı bulundu` | Yasak ayar | Ayarı kaldır |
| `Cucumber paralel koşumu açık` | Paralel koşum | `false` yap |
| `Source option 5 is no longer supported` | Eski derleme eklentisi | Adım 1, `maven-compiler-plugin` |
| `package javafx.embed.swing does not exist` | `javafx-swing` yok | Adım 1 |
| `cannot find symbol`, altında `class VideoKaydi` ya da `DivanKanit` | Paket adı farklı | Adım 3'ü doğru `PAKET` ile tekrarla |
| `sunucu 401 döndü` | `CI_API_TOKEN` yanlış | Adım 7 |
| `mvn.cmd: program not found` | Runner Maven'ı görmüyor | Runner'ı yeni terminalden başlat |
| `masaüstü oturumunda değil` | Runner Windows hizmeti | Hizmeti durdur, runner'ı masaüstünde başlat |
| `Yüklemede hiç Allure sonuç dosyası yok` | Testler başlamadı ya da eklenti çalışmadı | Önceki satırlardaki asıl hatayı bul |
| `xvfb-run: command not found` | Linux'ta Xvfb yok | `sudo apt-get install -y xvfb xauth` |

## Bitti listesi

- [ ] `ulak preflight` hatasız geçiyor, uyarı yok.
- [ ] Kalan bir senaryonun raporunda adımlar, ekran görüntüsü ve sahne ağacı var.
- [ ] Geçen senaryoda ekran görüntüsü yok.
- [ ] Canlı sayfada koşan senaryo ve adım görünüyor.
- [ ] Assertion hatası `failed`, diğer istisnalar `broken` görünüyor.

Ayrıntılar: `docs/TEST_PROJESI_GEREKSINIMLERI.md`. Çalışan örnek: `ornek-test-projesi/`.
