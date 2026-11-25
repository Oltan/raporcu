#!/usr/bin/env bash
# divan ev denemesi (Linux): GitLab olmadan tüm akışı gerçek JavaFX testleriyle koşturur.
#
#   ./ev-denemesi.sh            # 1 koşum
#   ./ev-denemesi.sh 3          # 3 koşum (dashboard'da gidişat grafiği ve flaky tespiti için)
#
# Gerekenler: Java 21, Maven, xvfb (sudo apt install openjdk-21-jdk maven xvfb xauth),
# isteğe bağlı ffmpeg (kalan senaryonun videosu). İlk koşumda Maven bağımlılıkları indirir (internet).
set -uo pipefail

KOSU=${1:-1}
PORT=${PORT:-8080}
TOKEN=ev-denemesi
KOK=$(cd "$(dirname "$0")" && pwd)

bul() { for p in "$@"; do [ -e "$p" ] && { echo "$p"; return; }; done; }
DIVAN=$(bul "$KOK/divan" "$KOK/../divan" "$KOK/../target/release/divan" "$KOK/../target/debug/divan")
ULAK=$(bul "$KOK/tools/ulak-linux" "$KOK/../tools/ulak-linux" "$KOK/../target/release/ulak" "$KOK/../target/debug/ulak")
PROJE_KAYNAK=$(bul "$KOK/ornek-test-projesi" "$KOK/../ornek-test-projesi")

hata() { echo "HATA: $*" >&2; exit 1; }
[ -n "$DIVAN" ] || hata "divan ikilisi bulunamadı (paketin kökünde ya da target/release altında olmalı)"
[ -n "$ULAK" ] || hata "ulak ikilisi bulunamadı (tools/ulak-linux)"
[ -n "$PROJE_KAYNAK" ] || hata "ornek-test-projesi klasörü bulunamadı"
command -v java >/dev/null || hata "java yok: sudo apt install openjdk-21-jdk"
command -v mvn >/dev/null || hata "maven yok: sudo apt install maven"
command -v xvfb-run >/dev/null || hata "xvfb yok: sudo apt install xvfb xauth"
command -v ffmpeg >/dev/null || echo "Not: ffmpeg yok, video kaydı olmadan devam ediliyor (sudo apt install ffmpeg)"
java -version 2>&1 | grep -q 'version "2[1-9]' || echo "UYARI: Java 21 ya da üstü gerekir: $(java -version 2>&1 | head -1)"
curl -s "http://127.0.0.1:$PORT/api/health" >/dev/null && hata "$PORT portu dolu. Başka port: PORT=8090 $0"

IS="$KOK/ev-denemesi-veri"          # her şey burada; silmek yeterli
mkdir -p "$IS"
echo "Çalışma klasörü: $IS"

# --- sunucu ---
export DATA_DIR="$IS/veri" BIND_ADDR="127.0.0.1:$PORT" PUBLIC_URL="http://127.0.0.1:$PORT" CI_API_TOKEN="$TOKEN" LOG_STDOUT=false
mkdir -p "$DATA_DIR/tools"
cp "$ULAK" "$DATA_DIR/tools/ulak-linux"
(cd "$IS" && exec "$DIVAN") > "$IS/sunucu.txt" 2>&1 &
SUNUCU=$!
trap 'kill $SUNUCU 2>/dev/null' EXIT
for _ in $(seq 50); do curl -s "$PUBLIC_URL/api/health" >/dev/null && break; sleep 0.2; done
curl -s "$PUBLIC_URL/api/health" >/dev/null || { cat "$IS/sunucu.txt"; hata "sunucu açılmadı"; }
echo "divan açıldı: $PUBLIC_URL"
ac() { command -v xdg-open >/dev/null && xdg-open "$1" >/dev/null 2>&1 || true; }
ac "$PUBLIC_URL/"

# --- test projesi (kopyası üzerinde; asıl klasör kirlenmez) ---
PROJE="$IS/ornek-test-projesi"
rm -rf "$PROJE"
cp -r "$PROJE_KAYNAK" "$PROJE"
rm -rf "$PROJE/target" "$PROJE/.ulak"
export ULAK_SERVER="$PUBLIC_URL" CI_PROJECT_PATH="ev/ornek-test-projesi" CI_COMMIT_REF_NAME="main"
cd "$PROJE"

"$ULAK" preflight || hata "ön kontrol geçmedi"
for n in $(seq "$KOSU"); do
  echo
  echo "=== Koşum $n / $KOSU ==="
  rm -f .ulak/run.json
  # koşum açılınca canlı ekranı tarayıcıda aç
  ( for _ in $(seq 300); do
      if [ -f .ulak/run.json ]; then
        url=$(sed -n 's/.*"report_url": *"\([^"]*\)".*/\1/p' .ulak/run.json)
        [ -n "$url" ] && { echo ">>> Canlı ekran: $url/live"; ac "$url/live"; }
        break
      fi
      sleep 0.2
    done ) &
  "$ULAK" start;  echo "(test işi çıkış kodu: $? — örnek projede bilerek kalan senaryolar var, 1 normaldir)"
  "$ULAK" upload
  "$ULAK" finish
done

echo
echo "Bitti. Dashboard: $PUBLIC_URL/"
echo "Admin (paylaşım, triage) için: kapatın, cd \"$IS\" && \"$DIVAN\" set-admin-password, sonra betiği yeniden çalıştırın."
echo "Sunucu açık kalıyor; kapatmak için Enter."
read -r _
