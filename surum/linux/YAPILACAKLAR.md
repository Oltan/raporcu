# Ubuntu'ya kurulum (yalnızca SSH ile)

> **Var olan bir kurulumu güncelliyorsan:** 0.1.0 → 0.1.3 `surum/GECIS_0.1.3.md`, 0.1.3 → 0.1.4 `surum/GECIS_0.1.4.md`, 0.1.4 → 0.1.5 `surum/GECIS_0.1.5.md`, 0.1.5 → 0.2.0 `surum/GECIS_0.2.0.md`; 0.2.0 → 0.2.3 yalnızca ikilileri değiştir (veritabanı, `.env` ve CI aynı).

Derleme: `depo-yapisi` dalı, 0.2.3, statik (musl): glibc'ye bağlı değil, her Ubuntu sürümünde çalışır.
Sağlama: `SHA256.txt`. Windows sürümüyle aynı kod (`surum/windows`).

**Her şey SSH'den yapılır, VNC gerekmez.** Testler Xvfb'nin sanal ekranında açılır; makinede masaüstü
ya da oturum açık olması gerekmez. Testleri izlemek için divan'ın canlı ekranı kendi tarayıcınızda
açılır (§6).

Komutlar Ubuntu 22.04 / 24.04 içindir. `<…>` yerlerini kendi değerlerinizle doldurun.

## 0. Dosyaları makineye kopyala (kendi bilgisayarınızdan)

Depo ya da kurulum paketi hangi bilgisayardaysa orada, PowerShell'de (Windows 10/11'de `scp` yerleşik):

```powershell
scp surum\linux\divan surum\linux\ulak surum\windows\ulak.exe .env.example deploy\linux\divan.service ci\gitlab\runner\config.toml.example <kullanici>@<makine>:/tmp/divan-kurulum/
```

Önce hedef klasörü açın: `ssh <kullanici>@<makine> mkdir -p /tmp/divan-kurulum`.
Java tarafı için kurulum paketindeki `java/` klasörü de gerekiyorsa (`m2`, destek jar'ı):
`scp -r java <kullanici>@<makine>:/tmp/divan-kurulum/`.

Sonra bağlanın: `ssh <kullanici>@<makine>`. Aşağıdakilerin hepsi o SSH oturumunda.

## 1. Paketler

```bash
sudo apt-get update
sudo apt-get install -y xvfb xauth ffmpeg openjdk-21-jdk maven git curl \
  libgtk-3-0 libgl1 libxtst6 libxrender1 libxi6 fonts-dejavu
sudo apt-get install -y libasound2t64 || sudo apt-get install -y libasound2   # 24.04 / 22.04
```

- Makine internete çıkamıyorsa `apt-get` şirketin paket aynası ya da vekil sunucusu (proxy) üzerinden
  çalışmalı. Proxy varsa: `echo 'Acquire::http::Proxy "http://<proxy>:<port>";' | sudo tee /etc/apt/apt.conf.d/90proxy`
- Kontrol: `java -version` (21), `mvn -version`, `xvfb-run --help`, `ffmpeg -version`.

## 2. divan sunucusu (hizmet)

```bash
cd /tmp/divan-kurulum
sudo useradd --system --home /var/lib/divan --shell /usr/sbin/nologin divan
sudo mkdir -p /opt/divan /var/lib/divan/tools
sudo install -m 755 divan /opt/divan/divan
sudo install -m 755 ulak /var/lib/divan/tools/ulak-linux
sudo install -m 644 ulak.exe /var/lib/divan/tools/ulak-windows.exe   # Windows runner'lar da bu sunucuyu kullanacaksa
sudo cp .env.example /opt/divan/.env
sudo nano /opt/divan/.env
```

`.env`'de değiştirilecekler:

| Ayar | Değer |
|---|---|
| `DATA_DIR` | `/var/lib/divan` |
| `BIND_ADDR` | yalnızca bu makine: `127.0.0.1:8080` (SSH tüneliyle açılır, §6); başkaları da açacaksa `0.0.0.0:8080` |
| `PUBLIC_URL` | raporlardaki linklerin adresi, ör. `http://<makine>:8080` |
| `CI_API_TOKEN` | uzun rastgele değer: `openssl rand -hex 32`. GitLab'a da aynısı yazılır |
| `GITLAB_WEBHOOK_TOKEN` | uzun rastgele değer |
| `GEREKSINIM_ONEKI`, `EK_KIMLIK_ONEKLERI` | `YTET_`, `YGÖ-` (dosya UTF-8; `nano` öyle kaydeder) |
| `JIRA_…`, `GITLAB_URL`, `GITLAB_TOKEN`, `SMTP_…` | kullanılacaksa |

```bash
cd /opt/divan && sudo ./divan set-admin-password      # şifreyi sorar, .env'e hash yazar
sudo chown -R divan:divan /opt/divan /var/lib/divan
sudo chmod 600 /opt/divan/.env
sudo cp /tmp/divan-kurulum/divan.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now divan
curl -s http://127.0.0.1:8080/api/health                 # {"durum":"ok",...}
```

Log: `journalctl -u divan -f` · Yeniden başlatma: `sudo systemctl restart divan`.

## 2b. Windows'taki divan'dan taşıma (Windows kapatılacaksa)

Veritabanı ve dosyalar işletim sisteminden bağımsız; klasörün tamamı taşınır. Geçmiş, triage kararları,
Jira bağları, paylaşım linkleri ve Admin → Ayarlar'daki şablonlar gelir.

1. **Windows'ta** divan'ı durdurun (pencereyi kapatın / hizmeti durdurun). Düzgün kapanınca veritabanı
   tek dosyaya yazılır (`divan.db`; yanında `-wal` kalmamalı).
2. **Windows'tan** kopyalayın (PowerShell; `backup` klasörü gerekmez, büyükse atlayın):
   ```powershell
   scp -r C:\divan\veri\divan.db C:\divan\veri\runs <kullanici>@<makine>:/tmp/divan-kurulum/veri/
   ```
   Önce Linux'ta: `mkdir -p /tmp/divan-kurulum/veri`. Eski `.env`'den `ADMIN_PASSWORD_HASH`,
   `CI_API_TOKEN`, `GITLAB_WEBHOOK_TOKEN`, `JIRA_…`, `GITLAB_…`, `SMTP_…`, `GEREKSINIM_ONEKI`,
   `EK_KIMLIK_ONEKLERI` satırlarını da yeni `.env`'e taşıyın (aynı token'lar = GitLab'da değişiklik yok).
3. **Linux'ta**, §2'deki hizmeti başlatmadan önce:
   ```bash
   sudo cp -r /tmp/divan-kurulum/veri/. /var/lib/divan/
   sudo chown -R divan:divan /var/lib/divan
   ```
   Sonra §2'nin geri kalanı (`set-admin-password` gerekmez, hash taşındıysa).
4. Adres değiştiği için:
   - `.env`'de `PUBLIC_URL` yeni adres olmalı.
   - GitLab'da Merge request webhook'u (Settings → Webhooks) yeni adrese: `http://<makine>:8080/api/gitlab/webhook`.
   - Eski MR açıklamalarındaki ve Jira bug'larındaki rapor linkleri eski adresi gösterir; Windows
     kapanınca açılmaz. Yeni rapor linkleri yeni adresle yazılır.

## 3. GitLab runner

```bash
curl -L "https://packages.gitlab.com/install/repositories/runner/gitlab-runner/script.deb.sh" | sudo bash
sudo apt-get install -y gitlab-runner
sudo gitlab-runner register --non-interactive \
  --url https://<gitlab-adresi>/ --token <glrt-…> \
  --executor shell --description divan-linux
sudo sed -i 's/^concurrent = .*/concurrent = 1/' /etc/gitlab-runner/config.toml
sudo rm -f /home/gitlab-runner/.bash_logout     # Ubuntu'da shell executor'ı "prepare environment" hatasıyla düşürür
sudo systemctl restart gitlab-runner
```

- Runner token'ını GitLab'da proje → Settings → CI/CD → Runners → "New project runner" ile alın;
  etiket **`linux`** olmalı.
- `packages.gitlab.com`'a erişim yoksa `.deb` dosyasını başka bir makinede indirip `scp` ile getirin ve
  `sudo apt-get install -y ./gitlab-runner_*.deb` ile kurun.
- Şirket GitLab'ı şirket CA'sıyla imzalıysa CA'yı sisteme tanıtın:
  `sudo cp sirket-ca.crt /usr/local/share/ca-certificates/ && sudo update-ca-certificates`.
- Runner hizmet olarak çalışır; oturum açma, ekran kilidi, uyku ayarı gerekmez.

## 4. Maven deposu (runner kullanıcısıyla)

Şirkette Nexus/Artifactory yoksa paketteki çevrimdışı depoyu runner'ın kullanıcısına kopyalayın:

```bash
sudo -u gitlab-runner mkdir -p /home/gitlab-runner/.m2/repository
sudo cp -r /tmp/divan-kurulum/java/m2/. /home/gitlab-runner/.m2/repository/
echo '<settings><offline>true</offline></settings>' | sudo tee /home/gitlab-runner/.m2/settings.xml
sudo chown -R gitlab-runner:gitlab-runner /home/gitlab-runner/.m2
```

Nexus varsa yalnızca destek jar'ını oraya yükleyin (`docs/KURULUM.md` §3 B).

## 5. Test projesi ve GitLab

1. Test projesinin `.gitlab-ci.yml`'ında `ULAK_OS: linux` (Windows runner'ı da varsa ayrı proje ya da
   ayrı değişken).
2. Proje CI değişkenleri: `CI_API_TOKEN` (masked, §2'dekiyle aynı), istenirse `ULAK_FIX_TOKEN`,
   `ULAK_FIX_PIPELINE=senaryo` (bkz. `surum/windows/YAPILACAKLAR.md` §5).
3. Self-healing (OpenCode / Claude Code) kullanılacaksa ajanın da bu makineye kurulması gerekir
   (Node.js + `opencode` ya da `claude`); `docs/KOD_AJANI_VLLM.md`. Kurulmadıysa önce `ULAK_FIX=off`.

**Elle deneme (pipeline'ı beklemeden):** test projesini runner kullanıcısıyla çekip bir kez koşturun:

```bash
sudo -u gitlab-runner -i
git clone <test-projesi-adresi> deneme && cd deneme
/var/lib/divan/tools/ulak-linux preflight
xvfb-run -a mvn -B test          # JavaFX sanal ekranda açılır; ekran gerekmez
```

## 6. Raporu ve canlı ekranı kendi bilgisayarınızdan açmak

`BIND_ADDR=127.0.0.1:8080` bıraktıysanız SSH tüneli kullanın (port dışarı açılmaz):

```powershell
ssh -L 8080:127.0.0.1:8080 <kullanici>@<makine>
```

Tünel açıkken tarayıcınızda `http://localhost:8080`. Testler koşarken canlı ekran, kalan senaryonun
ekran görüntüsü ve videosu buradan görünür; VNC'ye gerek yok.

## 7. Kontrol listesi

- [ ] `curl -s http://127.0.0.1:8080/api/health` → `"durum":"ok"`
- [ ] GitLab'da runner yeşil, etiketi `linux`
- [ ] `ulak-linux preflight` hata vermiyor
- [ ] Elle `xvfb-run -a mvn -B test` geçiyor (ya da yalnızca beklenen senaryolar kalıyor)
- [ ] Bir pipeline'da test → report → fix çalıştı, rapor linki iş çıktısında
- [ ] Tünelle rapor açılıyor; kalan senaryoda ekran görüntüsü ve video var

Güncelleme: `sudo systemctl stop divan`, yeni `divan`'ı `/opt/divan/`'a, yeni `ulak`'ı
`/var/lib/divan/tools/ulak-linux`'a kopyalayın, `sudo systemctl start divan`.

## 8. Birden çok kişi, her birinin kendi Linux VM'i

Tek merkezi divan (yukarıdaki §2), her VM'de yalnızca runner. VM'lere divan kurulmaz.

1. **Merkezi divan:** `.env`'de `BIND_ADDR=0.0.0.0:8080`, `PUBLIC_URL=http://<divan-makinesi>:8080`;
   güvenlik duvarında 8080 açık (`sudo ufw allow 8080/tcp`, ufw kullanılıyorsa).
2. **Her VM için GitLab'da runner oluşturun** (proje ya da grup → Settings → CI/CD → Runners → New runner):
   etiketler `linux,<kişinin-adı>` (ör. `linux,ahmet`); opencode kurulu ve modele erişen VM'lere
   ayrıca `ajan`. Verilen `glrt-…` token'ı not edin.
3. **Her VM'de** (SSH ile; `runner-kur.sh` ve isteğe bağlı `java/m2` kopyalandıktan sonra):
   ```bash
   sudo ./runner-kur.sh --gitlab https://<gitlab-adresi> --divan http://<divan-makinesi>:8080 --m2 ./java/m2
   ```
   Paketleri kurar, runner'ı kaydeder (token'ı sorar), `concurrent = 1` yapar, Maven deposunu kopyalar,
   divan'a erişimi ve ulak indirmesini dener. Tekrar çalıştırılabilir.
4. **Test projesinin `.gitlab-ci.yml`'ı:** `ornek.gitlab-ci.yml` (bu klasörde). Fark: `ULAK_SERVER`
   merkezi adres, işler `tags: [$RUNNER]` ile seçilir (varsayılan `linux` = boştaki herhangi bir VM),
   fix işi `ajan` etiketli VM'de koşar.
5. **Herkes kendi VM'inde:** Build → Pipelines → Run pipeline → değişken `RUNNER=ahmet`; kişisel gece
   schedule'ında da `RUNNER=ahmet`.

Raporlar ortak: aynı proje + branch'in koşumları hangi VM'den gelirse gelsin aynı geçmişe yazılır.
VM'ler arasında ortam farkı (Java sürümü, çözünürlük, uygulamanın bağlandığı servis) flaky gibi
görünebilir; kişisel denemeler için kişisel branch kullanın. Hangi VM'de koştuğu koşum sayfasındaki
GitLab iş linkinden görülür.
