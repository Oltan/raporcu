# divan-test-destek 1.1.0

Test projesinin içine giren Java kodu: `divan.destek.DivanKanit` (hata anında ekran görüntüsü + TestFX
sahne ağacı), `divan.destek.CanliOlayEklentisi` (canlı ekran), `divan.destek.VideoKaydi`.
Kaynak: `eklentiler/java`. Sağlama: `SHA256.txt`.

**1.0.0'dan farkı:** sahne ağacı sekme (`TabPane`), kaydırma alanı (`ScrollPane`), `TitledPane`,
`SplitPane`, `Accordion`, `ToolBar`, `ButtonBar` içeriğine iniyor; sekmeler seçili olup olmadığıyla
yazılıyor; ekran görüntüsü alınamasa da ağaç ekleniyor. Bunlar olmadan ajan bu kontrollerin
`#fxId`'lerini göremiyor.

Hangi jar'ın kullanıldığını anlamak için: `unzip -p divan-test-destek-*.jar META-INF/MANIFEST.MF`
→ `Implementation-Version` ve `Divan-Commit`.

## Test projesine koymak (Nexus olmadan, iki yol)

### A) Jar'ı test reposunun içinde tutmak (önerilen: her runner VM'de ayrıca iş gerekmez)

Test reposunda bir klasör Maven deposu gibi kullanılır:

```bash
mvn install:install-file -Dfile=divan-test-destek-1.1.0.jar \
  -DgroupId=com.divan -DartifactId=divan-test-destek -Dversion=1.1.0 -Dpackaging=jar \
  -DlocalRepositoryPath=ci/m2repo
git add ci/m2repo
```

`pom.xml`'e:

```xml
<repositories>
  <repository>
    <id>proje-ici</id>
    <url>file://${project.basedir}/ci/m2repo</url>
  </repository>
</repositories>
```

ve bağımlılığın sürümü `1.1.0` (`pom-eklentisi.xml`). `ci/settings.xml` ile `<mirrorOf>*</mirrorOf>`
kullanılıyorsa aynanın `!proje-ici` ile bu depoyu dışarıda bırakması gerekir
(`<mirrorOf>*,!proje-ici</mirrorOf>`).

### B) Her runner'ın Maven deposuna kurmak

Her runner VM'de, runner kullanıcısıyla:

```bash
sudo -u gitlab-runner mvn install:install-file -Dfile=divan-test-destek-1.1.0.jar \
  -DgroupId=com.divan -DartifactId=divan-test-destek -Dversion=1.1.0 -Dpackaging=jar
```

ve `pom.xml`'deki bağımlılık sürümü `1.1.0`. VM sayısı arttıkça A yolu daha az iş çıkarır.

## Kaynak olarak kopyalandıysa

Sınıflar jar yerine projeye kaynak olarak kopyalandıysa (`docs/LLM_JAVA_ENTEGRASYON.md` adım 3),
`eklentiler/java/src/main/java/divan/destek/` altındaki üç dosyayı paket adını değiştirerek yeniden
kopyalayın. İkisini birlikte yapmayın: sınıflar iki kez tanımlanır.
