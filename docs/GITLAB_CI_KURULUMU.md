# GitLab CI kurulum rehberi (SW-10.1)

Bu rehber bir test sanal makinesini (Ubuntu ya da Windows) sıfırdan hazırlar: raporcu sunucusu,
GitLab runner, tokenlar ve gece zamanlaması. Sunucu ve runner **aynı makinede** çalışır; CI işleri
raporcu'ye `http://127.0.0.1:8080` adresinden ulaşır.

İçindekiler:

1. [İkilileri derleme](#1-ikilileri-derleme)
2. [Ubuntu sanal makine](#2-ubuntu-sanal-makine)
3. [Windows sanal makine](#3-windows-sanal-makine)
4. [Tokenlar ve GitLab ayarları](#4-tokenlar-ve-gitlab-ayarlari)
5. [Gece zamanlaması](#5-gece-zamanlamasi-sw-13)
6. [Doğrulama](#6-dogrulama)

---

## 1. İkilileri derleme

Geliştirme makinesinde (Linux):

```bash
rustup target add x86_64-pc-windows-gnu x86_64-unknown-linux-musl
sudo apt-get install -y gcc-mingw-w64-x86-64 musl-tools
# Linux: statik (musl) — glibc'ye bağlı değil, her Ubuntu sürümünde çalışır
cargo build --release --target x86_64-unknown-linux-musl -p raporcu -p stajci
cargo build --release --target x86_64-pc-windows-gnu -p raporcu -p stajci
```

`cargo build --release` (hedefsiz) de çalışır, ama ikili derlendiği makinenin glibc sürümünü
ister (ör. Ubuntu 24.04'te derlenen, 22.04'te açılmaz). Dağıtım için musl derlemesini kullanın.

Çıktılar:

| Dosya | Nereye |
|-------|--------|
| `target/x86_64-unknown-linux-musl/release/raporcu` | Ubuntu sunucu |
| `target/x86_64-unknown-linux-musl/release/stajci` | sunucuda `tools/stajci-linux` |
| `target/x86_64-pc-windows-gnu/release/raporcu.exe` | Windows sunucu |
| `target/x86_64-pc-windows-gnu/release/stajci.exe` | sunucuda `tools/stajci-windows.exe` |

Her iki `stajci` ikilisi her sunucunun `DATA_DIR/tools/` (ya da `STAJCI_DIR`) klasörüne konur.
CI şablonu aracı her koşumda `GET /api/tools/stajci?os=linux|windows` ucundan indirir; sunucu
güncellenince runner'lar ertesi koşumda yeni `stajci`'yi kullanır (SW-10.9).

## 2. Ubuntu sanal makine

### 2.1 Paketler

```bash
sudo apt-get update
sudo apt-get install -y xvfb xauth ffmpeg openjdk-21-jdk maven git curl \
  libgtk-3-0 libgl1 libxtst6 libxrender1 libxi6 libasound2t64 fonts-dejavu
```

- `xvfb`, `xauth`: sanal X11 ekranı (SW-1.2). Wayland kullanılmaz.
- `ffmpeg`: başarısız senaryoların videosu (`x11grab`).
- GTK/GL kütüphaneleri: JavaFX'in X11'de açılması için.

### 2.2 raporcu sunucusu (hizmet)

```bash
sudo useradd --system --home /var/lib/raporcu --shell /usr/sbin/nologin raporcu
sudo mkdir -p /opt/raporcu /var/lib/raporcu/tools
sudo cp raporcu /opt/raporcu/
sudo cp stajci /var/lib/raporcu/tools/stajci-linux
sudo cp stajci.exe /var/lib/raporcu/tools/stajci-windows.exe
sudo cp .env.example /opt/raporcu/.env    # düzenleyin: CI_API_TOKEN, GITLAB_WEBHOOK_TOKEN, PUBLIC_URL...
cd /opt/raporcu && sudo ./raporcu set-admin-password   # admin şifresi .env'e argon2 hash olarak yazılır
sudo chown -R raporcu:raporcu /opt/raporcu /var/lib/raporcu
sudo cp deploy/linux/raporcu.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now raporcu
curl http://127.0.0.1:8080/api/health
```

`raporcu.service` içinde `MemoryMax=150M` sert bellek sınırıdır (SW-10.6). Veritabanı ilk
açılışta `DATA_DIR/raporcu.db` olarak kendiliğinden oluşur; ayrı kurulum yoktur (SW-2.5).

### 2.3 GitLab runner (shell executor, hizmet olarak)

```bash
curl -L "https://packages.gitlab.com/install/repositories/runner/gitlab-runner/script.deb.sh" | sudo bash
sudo apt-get install -y gitlab-runner
sudo gitlab-runner register --non-interactive \
  --url https://gitlab.sirket.local/ --token glrt-XXXX \
  --executor shell --description raporcu-linux
```

`/etc/gitlab-runner/config.toml` içinde `concurrent = 1` olmalı (SW-1.7), örnek:
`ci/gitlab/runner/config.toml.example`. Runner etiketi GitLab'da **`linux`** olmalı.

Runner sistem hizmeti olarak çalışabilir: oturum açma, ekran kilidi ya da uyku ayarı gerekmez.
Her test işi kendi Xvfb ekranını `xvfb-run -a -s "-screen 0 1920x1080x24"` ile alır; aynı anda
iki iş koşsa ekranlar karışmaz (`-a` boş ekran numarası seçer). Bunu `stajci start` yapar
(`STAJ_XVFB=on`, çözünürlük `STAJ_SCREEN`).

## 3. Windows sanal makine

TestFX fare ve klavye olaylarını **oturum açmış kullanıcının masaüstüne** gönderir. Bu yüzden
GitLab runner Windows hizmeti olarak değil, masaüstü oturumunda çalışır (SW-1.6). Headless ya da
Monocle kullanılmaz.

### 3.1 Programlar

- Java 21 (Temurin) ve Maven, `PATH` içinde (`mvn.cmd`).
- ffmpeg (`gdigrab` ile video), `PATH` içinde.
- Git for Windows.
- `curl.exe` Windows 10/11'de hazır gelir (stajci indirmek için).

### 3.2 Oturum, kilit, uyku

Yönetici PowerShell'de:

```powershell
.\deploy\windows\kurulum.ps1 -Kullanici testrunner -RunnerDir C:\GitLab-Runner -StajDir C:\raporcu
```

Betik şunları yapar:

- uyku, ekran kapanması, hazırda bekleme kapalı (`powercfg`);
- ekran koruyucu ve kilit ekranı kapalı;
- **GitLab runner**: oturum açılınca başlayan zamanlanmış görev (`gitlab-runner run`), hizmet değil;
- **raporcu sunucusu**: makine açılınca SYSTEM olarak başlayan zamanlanmış görev (oturum gerekmez).

Elle yapılacaklar:

1. Otomatik oturum açma: Sysinternals Autologon (`Autologon64.exe testrunner MAKINE <şifre> /accepteula`).
   Makine yeniden başlayınca kullanıcı oturumu açılır, runner görev olarak başlar.
2. Ekran çözünürlüğünü sabitleyin (Ayarlar → Ekran, ör. 1920×1080). Sanal makine konsolunun
   çözünürlüğü de aynı olmalı.
3. Runner kaydı: `gitlab-runner.exe register --executor shell --shell powershell ...`, etiket
   **`windows`**, `config.toml` içinde `concurrent = 1`.
4. `C:\raporcu\` altına `raporcu.exe`, `.env` ve `tools\stajci-linux`, `tools\stajci-windows.exe`.
   `.\raporcu.exe set-admin-password` ile admin şifresi.

### 3.3 Uzak Masaüstü (RDP) ile bağlanırken

RDP bağlantısı kesilince oturum kilitlenir ve testler fare/klavye olaylarını gönderemez. RDP'den
çıkarken pencereyi kapatmak yerine **`deploy\windows\rdp-cikis.cmd`** betiğini yönetici olarak
çalıştırın: oturum `tscon <oturum> /dest:console` ile konsola geri verilir, masaüstü açık kalır.

`stajci start` runner'ın hizmet oturumunda (SYSTEM / `Services`) çalıştığını tespit ederse işi
açık bir mesajla düşürür.

## 4. Tokenlar ve GitLab ayarları

| Değer | Nerede | Ne için |
|-------|--------|---------|
| `CI_API_TOKEN` | raporcu `.env` + test projesinde (ya da grupta) masked CI değişkeni | CI uçları (SW-9.1) |
| `GITLAB_WEBHOOK_TOKEN` | raporcu `.env` + GitLab webhook "Secret token" | webhook doğrulama (SW-9.3) |
| `GITLAB_URL`, `GITLAB_TOKEN` | yalnızca raporcu `.env` | "OpenCode ile düzelt" düğmesi pipeline tetikler (SW-7.4) |
| `STAJ_FIX_TOKEN` | test projesinde masked CI değişkeni (proje erişim tokenı, `api` + `write_repository`) | `stajci fix`: auto-fix branch push + MR (SW-7.2) |
| `STAJ_FIX_AGENT` | CI değişkeni: `opencode` (varsayılan) ya da `claude` | self-healing ajanı |
| `STAJ_FIX_PIPELINE` | CI değişkeni (isteğe bağlı), varsayılan kapalı | `on`: auto-fix branch'ine push'ta da tüm e2e koşar (fix işi yine koşmaz) |
| `STAJ_OPENCODE_AGENT` | CI değişkeni (isteğe bağlı), varsayılan `fix` | `.opencode/agents/<ad>.md` ajan adı |
| `VLLM_BASE_URL`, `VLLM_API_KEY`, `TESTFX_MCP_JAR` | runner ortamı ya da CI değişkeni | OpenCode + vLLM (SW-7.6) |
| `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_MODEL`, `ANTHROPIC_DEFAULT_HAIKU_MODEL` | runner ortamı ya da CI değişkeni | Claude Code + vLLM (ağ geçidi üzerinden), `docs/KOD_AJANI_VLLM.md` |

Webhook: test projesi → Settings → Webhooks → URL `http://<raporcu-makinesi>:8080/api/gitlab/webhook`,
Secret token = `GITLAB_WEBHOOK_TOKEN`, tetikler: **Pipeline events** ve **Merge request events**.

GitLab token yalnızca `.env` içinde durur; loglara ve hata mesajlarına yazılmaz (SW-9.2).

## 5. Gece zamanlaması (SW-1.3)

Test projesi → Build → Pipeline schedules → New schedule:

- Interval: `0 2 * * *` (her gece 02:00), saat dilimi `Europe/Istanbul`
- Target branch: `main`
- Değişken (isteğe bağlı): `STAJ_TAGS` = gece koşulacak tag ifadesi

Schedule ile başlayan pipeline bitince raporcu pipeline başına tek özet e-posta gönderir
(`MAIL_TO`, SW-5.3). Rapor sabah raporcu dashboard'unda hazırdır.

## 6. Doğrulama

Windows ikilileri Linux'ta Wine ile de denenebilir (`.cargo/config.toml` içinde `runner = "wine"`):

```bash
sudo apt-get install -y wine64
cargo test --target x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu -p raporcu -p stajci -p rehearse -p e2esmoke
cd target/x86_64-pc-windows-gnu/debug && STAJ_SKIP_BUILD=1 wine rehearse.exe && STAJ_SKIP_BUILD=1 wine e2esmoke.exe
```

Wine gerçek Windows masaüstü oturumunun yerini tutmaz: JavaFX/TestFX'in Windows'ta gerçek
ekranda koşması ve otomatik oturum/RDP davranışı pilot VM'de doğrulanmalıdır.


Geliştirme makinesinde:

```bash
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p rehearse
cargo run -p e2esmoke
```

Kurulmuş sunucuya karşı smoke:

```bash
STAJ_SERVER=http://127.0.0.1:8080 CI_API_TOKEN=<token> cargo run -p e2esmoke
```

GitLab tarafı için `docs/PILOT_KONTROL_LISTESI.md`.
