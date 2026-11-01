# divan kurulum raporu

Bu paket, divan'ı derleme yapmadan ve Maven için internet gerektirmeden kurmak içindir. Sunucu ve
GitLab runner aynı makinede çalışır; makine Windows ya da Ubuntu olabilir. Sırayı izleyin, her
bölümün sonundaki **Kontrol** geçmeden sonrakine geçmeyin.

## Paketin içeriği

| Klasör | İçerik |
|--------|--------|
| `windows/` | `divan.exe` (sunucu), `ulak.exe` (CI aracı). Ek çalışma zamanı istemez |
| `linux/` | `divan`, `ulak`. Statik derlendi, her Ubuntu sürümünde çalışır |
| `java/` | `divan-test-destek-1.0.0.jar`, `m2/` (çevrimdışı Maven deposu), `maven/` (Maven 3.9.16), `pom-eklentisi.xml` |
| `test-projesi/` | `.gitlab-ci.yml`, `opencode.json`, `AGENTS.md`, `fix` ajanı, üç skill |
| `ozel/` | Kendi ajanlarınız ve skill'leriniz, örnekleriyle |
| `sunucu/` | `.env` örneği, Windows hazırlık betiği, Linux hizmet dosyası, ayrıntılı rehberler |
| `projeye-ekle.ps1`, `.sh` | Dosyaları test projesine kopyalayan betik |
| `SHA256.txt` | Aktarım sonrası dosya doğrulama listesi |

**Java tarafında neden exe yok:** Testleriniz çalışırken `mvn test` ile derlenir, bu yüzden tek bir
exe olamaz. Derlenmesi gereken ortak parça jar olarak hazırdır, bağımlılıklar `java/m2` içindedir.
Hedef makinede JDK gerekir, internet gerekmez.

## Kurulumdan önce gerekenler

| Gereken | Not |
|---------|-----|
| JDK 21 | Pakette yok. Destek jar'ı Java 17'yle uyumludur |
| GitLab Runner | Pakette yok. Shell executor. Sürümü GitLab sunucunuza uygun olmalı |
| Git | Runner'ın PATH'inde |
| ffmpeg | İsteğe bağlı, kalan senaryoların videosu için |
| Linux'ta Xvfb | `xvfb xauth` ve JavaFX kütüphaneleri (`sunucu/GITLAB_CI_KURULUMU.md` bölüm 2.1) |
| OpenCode | Self-healing için. 1.19.5 ile denendi |

Monocle ya da headless JavaFX kullanılamaz. Linux'ta Xvfb, Windows'ta açık masaüstü oturumu gerekir.

## 1. Sunucu

### Windows

Yönetici PowerShell'de, paket klasöründe:

```powershell
New-Item -ItemType Directory -Force C:\divan\veri\tools
Copy-Item windows\divan.exe C:\divan\
Copy-Item windows\ulak.exe C:\divan\veri\tools\ulak-windows.exe
Copy-Item linux\ulak C:\divan\veri\tools\ulak-linux
Copy-Item sunucu\env.ornek C:\divan\.env
notepad C:\divan\.env
cd C:\divan
.\divan.exe set-admin-password
.\divan.exe
```

### Ubuntu

```bash
sudo useradd --system --home /var/lib/divan --shell /usr/sbin/nologin divan
sudo mkdir -p /opt/divan /var/lib/divan/tools
sudo cp linux/divan /opt/divan/
sudo cp linux/ulak /var/lib/divan/tools/ulak-linux
sudo cp windows/ulak.exe /var/lib/divan/tools/ulak-windows.exe
sudo cp sunucu/env.ornek /opt/divan/.env
sudo nano /opt/divan/.env
cd /opt/divan && sudo ./divan set-admin-password
sudo chown -R divan:divan /opt/divan /var/lib/divan
sudo cp sunucu/divan.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now divan
```

### `.env` içinde değiştirilecekler

| Ayar | Değer |
|------|-------|
| `DATA_DIR` | Windows: `C:/divan/veri`, Ubuntu: `/var/lib/divan` |
| `BIND_ADDR` | Yalnızca bu makine: `127.0.0.1:8080`. Başkaları da açacaksa: `0.0.0.0:8080` |
| `PUBLIC_URL` | Linklerde görünecek adres. Başkaları açacaksa makinenin ağ adresi olmalı |
| `CI_API_TOKEN` | Uzun, rastgele bir değer. GitLab'a da aynısı yazılır |
| `GITLAB_WEBHOOK_TOKEN` | Uzun, rastgele bir değer |
| `GITLAB_URL`, `GITLAB_TOKEN` | "OpenCode ile düzelt" düğmesi için |

Token'lar yalnızca `.env` ve GitLab CI değişkeninde durur, koda ve belgelere yazılmaz.

**Kontrol:** Tarayıcıda `http://<adres>:8080/api/health` adresi `"durum":"ok"` döner.

## 2. GitLab runner

- Shell executor, `concurrent = 1` (`sunucu/config.toml.example`).
- Etiket Windows'ta `windows`, Ubuntu'da `linux` olmalı.
- **Windows:** Runner hizmet olarak çalışmamalı, masaüstü oturumunda başlamalı. Kilit, uyku ve görev
  ayarlarını `sunucu\kurulum.ps1` yapar:

```powershell
.\sunucu\kurulum.ps1 -Kullanici testrunner -RunnerDir C:\GitLab-Runner -DivanDir C:\divan
```

- RDP'den çıkarken pencereyi kapatmayın, `sunucu\rdp-cikis.cmd` çalıştırın. Yoksa oturum kilitlenir ve
  testler fare ile klavyeyi kullanamaz.

**Kontrol:** GitLab'da runner yeşil görünür.

## 3. Java ve Maven

Runner'ın kullanıcısıyla yapın. Depo kullanıcıya özeldir: Linux'ta çoğunlukla
`/home/gitlab-runner/.m2`, Windows'ta `C:\Users\testrunner\.m2`.

**A) Şirkette Nexus ya da Artifactory yoksa:** Depoyu kullanıcının Maven deposuna kopyalayın.

```powershell
New-Item -ItemType Directory -Force $HOME\.m2\repository
Copy-Item java\m2\* $HOME\.m2\repository -Recurse -Force
Copy-Item java\maven C:\divan\maven -Recurse
```

```bash
mkdir -p ~/.m2/repository
cp -r java/m2/. ~/.m2/repository/
```

`C:\divan\maven\bin` klasörünü runner'ın PATH'ine ekleyin. Linux'ta paketteki Maven'ı da aynı şekilde
PATH'e ekleyebilir ya da sistemin Maven'ını kullanabilirsiniz (3.8.7 ve 3.9.16 denendi).

Maven'ın dışarı çıkmaya çalışmaması için `~/.m2/settings.xml`:

```xml
<settings><offline>true</offline></settings>
```

**B) Nexus varsa:** Yalnızca jar'ı yükleyin, gerisini Nexus sağlar.

```bash
mvn deploy:deploy-file -Dfile=java/divan-test-destek-1.0.0.jar -DgroupId=com.divan -DartifactId=divan-test-destek -Dversion=1.0.0 -Dpackaging=jar -DrepositoryId=<id> -Durl=<nexus-adresi>
```

**Önemli sınır:** `m2` yalnızca örnek projedeki sürümleri içerir (Cucumber 7.34.9, TestFX 4.0.18,
Allure 2.35.5, JavaFX 21.0.10, JUnit 5.14.4). Projeniz başka sürüm ya da başka kütüphane kullanıyorsa
onlar da depoda olmalıdır. İnternetli bir makinede projenizde `mvn dependency:go-offline test`
çalıştırıp `.m2\repository` içeriğini aynı şekilde taşıyın.

**Kontrol:** Test projesinde `mvn -B -q test-compile` hatasız biter.

## 4. Test projesini bağlama

```powershell
.\projeye-ekle.ps1 -Proje C:\isler\test-projesi -Os windows
```

```bash
bash projeye-ekle.sh /isler/test-projesi linux
```

Betik var olan dosyaya dokunmaz, "ATLANDI" der. Projede `.gitlab-ci.yml` varsa `test-projesi`
içindekini elle birleştirin. Ezmek için `-Ustune` (Linux'ta üçüncü argüman `ustune`).

Sonra üç değişikliği elle yapın:

1. `java/pom-eklentisi.xml` içindeki parçaları `pom.xml`'e ekleyin.
2. `src/test/resources/junit-platform.properties`:

   ```properties
   cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm, divan.destek.CanliOlayEklentisi
   cucumber.execution.parallel.enabled=false
   ```

3. Hook sınıfı:

   ```java
   import divan.destek.DivanKanit;
   import divan.destek.VideoKaydi;

   private VideoKaydi video;

   // @Before içinde, uygulamayı açmadan önce
   video = VideoKaydi.baslat();

   // @After içinde, pencereyi kapatmadan önce. Metoda Scenario senaryo parametresi ekleyin.
   DivanKanit.ekle(senaryo, stage);
   if (video != null) {
       byte[] kayit = video.durdur(senaryo.isFailed());
       if (kayit != null) {
           senaryo.attach(kayit, "video/mp4", "Video");
       }
   }
   ```

`features.txt` dosyasına koşulacak tag'leri yazın (her satıra bir tag ifadesi, ör. `@smoke`).
`.gitignore` dosyasına `target/`, `.ulak/`, `ulak.env`, `ulak`, `ulak.exe` ekleyin.

Kişisel masaüstünde testleri koşacaksanız `.gitlab-ci.yml` içinde `ULAK_VIDEO: "false"` yapın:
Windows'ta video tüm masaüstünü kaydeder.

**Kontrol:** Proje kökünde paketteki ulak ile:

```powershell
<paket>\windows\ulak.exe preflight
```

```bash
<paket>/linux/ulak preflight
```

Son satır `ön kontrol geçti` olmalı. `UYARI` satırlarını da kapatın.

## 5. GitLab ayarları

Test projesi → Settings → CI/CD → Variables:

| Değişken | Değer |
|----------|-------|
| `CI_API_TOKEN` | Masked. `.env` içindekiyle aynı |
| `ULAK_FIX_TOKEN` | Masked. Proje erişim token'ı, `api` + `write_repository`. Self-healing MR açmak için |
| `ULAK_FIX` | Self-healing kurulana kadar `off` |

Webhook: `http://<divan-adresi>:8080/api/gitlab/webhook`, Secret token `GITLAB_WEBHOOK_TOKEN`,
tetikler **Pipeline events** ve **Merge request events**. Gece koşumu için pipeline schedule ekleyin.
Jira ve DOORS: `sunucu/JIRA_DOORS_AYARLARI.md`.

**Kontrol:** Bir push sonrası divan'ın **Canlı** sayfası koşan senaryoyu gösterir, koşum `bitti` olur.

## 6. OpenCode modeli

Evet, model tamamen ayarlanabilir. OpenAI uyumlu her sunucu çalışır (vLLM, LiteLLM, Ollama vb.).
Dosya değiştirmeniz gerekmez, modeli runner ortamından verirsiniz:

| Değişken | Örnek | Anlamı |
|----------|-------|--------|
| `VLLM_BASE_URL` | `http://llm.sirket.local:8000/v1` | Model sunucusu, sonu `/v1` |
| `VLLM_API_KEY` | | Anahtar. Sunucu istemiyorsa herhangi bir değer. Masked |
| `VLLM_MODEL` | `qwen3-coder` | Sunucudaki tam model adı. `/` ve `:` içerebilir |
| `ULAK_OPENCODE_MODEL` | `vllm/baska-model` | İsteğe bağlı. Her şeyi ezer, deneme için |
| `ULAK_OPENCODE_BIN` | `C:\araclar\opencode.exe` | Windows'ta gerekli, aşağıya bakın |
| `TESTFX_MCP_JAR` | | İsteğe bağlı. TestFX MCP sunucusu varsa jar yolu |

Bunları CI değişkeni olarak ya da runner `config.toml` içindeki `environment` satırına yazın.

- **Windows:** npm yalnızca kısayol koyar, `ulak` onu çalıştıramaz. Gerçek yolu şöyle bulun ve
  `ULAK_OPENCODE_BIN` yapın:

  ```powershell
  Join-Path (npm root -g) "opencode-ai\bin\opencode.exe"
  ```

- **Görsel desteklemeyen model:** `opencode.json` içinde `"attachment": false` yapın ve
  `ULAK_FIX_CMD` ile `--file` bayrağını kaldırın:

  ```text
  opencode run --agent fix {istem}
  ```

  `ULAK_FIX_CMD` verilirse `ULAK_OPENCODE_BIN` ve `ULAK_OPENCODE_MODEL` kullanılmaz; yolu ve modeli
  komutun içine yazın. Ekran görüntüsünün yolu istemde yine yazar.
- **Bağlam sınırı:** `opencode.json` içinde `limit.context` 32768, `limit.output` 4096. Modelinize
  göre değiştirin.
- **Araç çağırma şart:** Model dosya okuma ve düzenlemeyi araç çağrısıyla yapar. vLLM'de
  `--enable-auto-tool-choice` ve uygun `--tool-call-parser` açık olmalı.

**Kontrol:** Test projesinde, ortam değişkenleri tanımlıyken:

```bash
opencode run --agent fix "Merhaba de"
```

Yanıt gelmeli ve başlık satırında sizin model adınız görünmeli. Ardından `ULAK_FIX` değişkenini
silin, bilinen bir test kodu hatasıyla pipeline koşun. Sonuç koşum sayfasındaki OpenCode kutusunda
görünür. Ayrıntı: `sunucu/KOD_AJANI_VLLM.md`.

## 7. Kendi ajanlarınız ve skill'leriniz

**Dosyaları nereye koyacaksınız:**

| Ne | Paket içinde | Test projesinde |
|----|--------------|-----------------|
| Ajan | `ozel/agents/<ad>.md` | `.opencode/agents/<ad>.md` |
| Skill | `ozel/skills/<ad>/SKILL.md` | `.claude/skills/<ad>/SKILL.md` |

`projeye-ekle` ikisini de kopyalar. Her proje kendi kopyasını taşır; değişiklik o projenin deposuna
commit edilir. Örnekler `ozel/agents/ORNEK-ajan.md` ve `ozel/skills/ornek-skill/` (bunlar projeye
kopyalanmaz). Skill'lere yardımcı dosya da ekleyebilirsiniz, klasörün tamamı kopyalanır.

**Ajanı devreye almak:** CI değişkeni `ULAK_OPENCODE_AGENT=<ad>`. Ayarlanmazsa `fix` çalışır.

**Ajanın uyması gereken kurallar** (bozulursa self-healing sessizce değersizleşir):

1. Başlıkta `mode: primary`. Soru soramaz: `permission` alanlarında `ask` kullanmayın.
2. `bash: deny` önerilir. Doğrulama koşusunu `ulak` yapar.
3. `skill: allow` olmalı, yoksa skill'ler yüklenmez.
4. Yalnızca `src/test/` değiştirilmeli. `ulak` dışına çıkan değişikliği geri alır ve reddeder.
5. Son iki satır tam olarak şu biçimde olmalı:

   ```text
   ULAK_OZET: <tek cümlelik açıklama>
   ULAK_TRIAGE: {"category": "test_kodu|urun|ortam|flaky|bilinmiyor", "reason": "...", "confidence": 0.85}
   ```

   `urun` kararı güven 0,70 ve üstündeyse divan Jira bug taslağı hazırlar.

**Skill kuralları:**

```markdown
---
name: sayfa-nesneleri
description: Projedeki sayfa nesnesi sınıfları. Yeni seçici ya da adım yazarken yükle.
---

# İçerik: kurallar, örnek kod, yapılmayacaklar
```

- `name` klasör adıyla aynı olmalı: küçük harf, rakam, tek tire.
- `description` skill'in **ne zaman** yükleneceğini söylemeli. Ajan buna bakarak seçer.
- Skill'i ajan talimatında adıyla da anın, güvenilirliği artar.

**Kontrol:** `ulak preflight` her ajan ve skill sorununu `UYARI` olarak yazar. Sonuçta
`OpenCode ajanı: <ad>` ve `<n> skill bulundu` satırlarını görmelisiniz.

## 8. Son kontrol listesi

- [ ] `/api/health` `ok` döndürüyor.
- [ ] Yönetici girişi yapılıyor.
- [ ] `ulak preflight` uyarısız geçiyor.
- [ ] Pipeline'da `Canlı` sayfası koşan adımı gösteriyor.
- [ ] Kalan senaryoda adımlar, ekran görüntüsü ve sahne ağacı var.
- [ ] Koşum sayfasında **Genel ana sayfa linki** üretiliyor, gizli pencerede giriş istemeden açılıyor.
- [ ] Self-healing denemesi koşum sayfasında sonuç gösteriyor.

## 9. Güncelleme

divan'ı durdurun, yeni `divan`/`divan.exe` ile `tools` altındaki `ulak` ikililerini değiştirin, yeniden
başlatın. Veritabanı güncellemeleri açılışta kendiliğinden uygulanır, önce yedek alınır. Runner'lar
yeni `ulak`'ı sonraki koşumda sunucudan indirir. Sunucu ve `ulak` sürümleri uyumlu olmalıdır.

## 10. Sorun giderme

| Görülen | Sebep | Çözüm |
|---------|-------|-------|
| `Cannot access central ... in offline mode` | Kütüphane depoda yok | Bölüm 3, önemli sınır |
| `mvn.cmd: program not found` | Runner Maven'ı görmüyor | PATH'i düzeltip runner'ı yeniden başlatın |
| `masaüstü oturumunda değil` | Runner Windows hizmeti | Runner'ı masaüstünde başlatın |
| `sunucu 401 döndü` | `CI_API_TOKEN` farklı | `.env` ve GitLab değerini eşitleyin |
| Çıkış kodu 2 | `ulak` ve sunucu sürümü farklı | `tools` klasörünü güncelleyin |
| Çıkış kodu 3 | Bellek yetmedi | `ULAK_TEST_XMX`, `ULAK_MAVEN_XMX` |
| `xvfb-run: command not found` | Xvfb yok | `xvfb xauth` kurun |
| `Allure adaptörü eksik` | `pom.xml` eksik | Bölüm 4 |
| `opencode ... program not found` | Windows kısayolu | `ULAK_OPENCODE_BIN` |
| OpenCode yanıt vermiyor | Adres ya da model adı yanlış | `VLLM_BASE_URL` sonu `/v1`, `VLLM_MODEL` tam ad |
| Sonuçlar gelmiyor, `.ulak/queue` doluyor | Sunucu kapalı | Sunucuyu açın, `ulak upload --resume` |

Java projesini küçük bir yerel modelle adım adım bağlamak için `sunucu/LLM_JAVA_ENTEGRASYON.md`,
daha ayrıntılı sunucu kurulumu için `sunucu/GITLAB_CI_KURULUMU.md`.
