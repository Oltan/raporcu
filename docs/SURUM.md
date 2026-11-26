# Sürüm çıkarmak (etiket)

Etiket **deponun tamamı** içindir: divan, ulak, Java eklentisi ve CI şablonu birlikte aynı numarayı taşır.
Biçim `vMAJOR.MINOR.YAMA` (ör. `v0.1.2`).

## Hangi hane artar?

| Değişiklik | Örnek | Artan |
|---|---|---|
| Sözleşme bozulmadı (divan ve ulak eski karşı tarafla çalışır) | hata düzeltme, yeni log satırı, yeni ayar | YAMA: `0.1.2` → `0.1.3` |
| Sözleşme değişti (uç, JSON alanı, canlı olay biçimi) | yeni zorunlu alan | MINOR: `0.1.x` → `0.2.0` |

divan ile ulak'ın **ilk iki hanesi aynı olmalı**; değilse divan ulak'ın isteğini reddeder ve iş 2 koduyla
düşer. MINOR artınca sunucu ve `tools/ulak-*` birlikte güncellenir.

Java eklentisinin (`eklentiler/java/pom.xml`) kendi sürümü vardır ve yalnızca Java kodu değişince artar;
`pom-eklentisi.xml`'deki sürüm onunla aynı olmalı (test denetler).

## Adımlar

1. **Sürüm numarası:** `Cargo.toml` → `[workspace.package] version`. Java değiştiyse
   `eklentiler/java/pom.xml` ve `eklentiler/java/pom-eklentisi.xml`.
2. **Kontrol:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
   `cargo run -p rehearse`.
3. **Derleme** (Linux makinede; araçlar: `rustup target add x86_64-pc-windows-gnu x86_64-unknown-linux-musl`,
   `apt-get install gcc-mingw-w64-x86-64 musl-tools`):
   ```bash
   cargo build --release --target x86_64-pc-windows-gnu -p divan -p ulak
   cargo build --release --target x86_64-unknown-linux-musl -p divan -p ulak
   mvn -B -f eklentiler/java/pom.xml clean package -Ddivan.commit=$(git rev-parse --short HEAD)
   ```
   Çıktıları `surum/windows`, `surum/linux`, `surum/java`'ya kopyalayıp `SHA256.txt`'leri yenileyin.
4. **Commit**, sonra **etiket** (açıklamalı) ve gönderme:
   ```bash
   git tag -a v0.1.3 -m "v0.1.3: <kısa açıklama>"
   git push origin v0.1.3
   ```
   Arayüzden: GitHub/GitLab → Releases (Tags) → yeni sürüm → etiket `v0.1.3`, hedef commit seçilir;
   açıklamaya değişiklikler yazılır, `surum/` altındaki dosyalar eklenebilir.
5. **Kurulum:** sunucuda `divan` ve `tools/ulak-linux` (`ulak-windows.exe`) değiştirilir, divan yeniden
   başlatılır; runner'lar yeni ulak'ı sonraki işte indirir. Java değiştiyse test projesindeki jar
   (`surum/java/README.md`).

## Hangi sürüm çalışıyor?

- divan: `curl -s http://<sunucu>:8080/api/health` → `"surum"`
- ulak: iş logunda istek başlığı; elle `ulak --version`
- Java eklentisi: `unzip -p divan-test-destek-*.jar META-INF/MANIFEST.MF` → `Implementation-Version`, `Divan-Commit`

## Etiket listesi

| Etiket | Commit | Not |
|---|---|---|
| `v0.1.0` | `33c04cb` | sahadaki ilk sürüm (YTET/YGÖ, Jira label, ULAK_FIX_PIPELINE=senaryo) |
| `v0.1.2` | `6d41925` | ulak zaman aşımında atlar, ayarları loga yazar; Cucumber dışı sonuçlar ajan dışı; divan-test-destek 1.1.0 |
| `v0.1.3` | `a3bf310` | uygulama sürümü raporda (migration 0007); `@ABC-123` Jira tag'leri, başlık/durum Jira'dan (0008); değerlendir/fix sınırı varsayılan sınırsız, değişken adı toleranslı; geçiş rehberi (ikililer `deaf05e`'de derlendi, kod aynı) |
| `v0.1.4` | `34d5f78` | elle Jira bağlama (senaryo sayfası, aynı hataya sonraki koşumlarda taşınır); auto-fix pipeline'ında build → test → report kendiliğinden (`ULAK_FIX_PIPELINE: senaryo`) |
| `v0.1.5` | `1943142` | uygulama sürümü dashboard'da ayrı sütun ve her raporda (yoksa `—`, boşsa pom'daki); raporda, ekip/genel linkinde ve e-postada bağlı Jira issue'ları |
