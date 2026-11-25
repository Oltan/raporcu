#!/usr/bin/env bash
# divan için Linux test VM'i hazırlar: paketler, GitLab runner (shell executor), Maven deposu.
# Sunucu (divan) kurmaz: sonuçlar merkezi divan'a gider. Tekrar çalıştırılabilir.
#
# Kullanım (VM'de, SSH ile):
#   sudo ./runner-kur.sh --gitlab https://gitlab.sirket.local --divan http://divan.sirket.local:8080 [--m2 /yol/java/m2]
# Runner token'ı sorulur (ekrana yazılmaz) ya da RUNNER_TOKEN ortam değişkeniyle verilir.
# Etiketler (ör. "linux" ve kişinin adı) GitLab'da runner oluşturulurken verilir.
set -euo pipefail

GITLAB=""; DIVAN=""; M2=""
while [ $# -gt 0 ]; do
  case "$1" in
    --gitlab) GITLAB="${2%/}"; shift 2 ;;
    --divan)  DIVAN="${2%/}"; shift 2 ;;
    --m2)     M2="$2"; shift 2 ;;
    *) echo "bilinmeyen seçenek: $1"; exit 2 ;;
  esac
done
[ "$(id -u)" -eq 0 ] || { echo "HATA: sudo ile çalıştırın"; exit 1; }
if [ -z "$GITLAB" ] || [ -z "$DIVAN" ]; then echo "HATA: --gitlab ve --divan gerekli"; exit 2; fi

adim() { printf '\n== %s\n' "$1"; }

adim "1/6 Paketler (Xvfb, Java 21, Maven, ffmpeg, JavaFX kütüphaneleri)"
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q xvfb xauth ffmpeg openjdk-21-jdk maven git curl \
  libgtk-3-0 libgl1 libxtst6 libxrender1 libxi6 fonts-dejavu
apt-get install -y -q libasound2t64 2>/dev/null || apt-get install -y -q libasound2

adim "2/6 Merkezi divan erişimi"
if curl -fsS --max-time 10 "$DIVAN/api/health" | grep -q '"ok"'; then
  echo "divan erişilebilir: $DIVAN"
else
  echo "UYARI: $DIVAN/api/health yanıt vermedi (divan BIND_ADDR=0.0.0.0, güvenlik duvarında port açık mı?)"
fi
TMPU=$(mktemp)
if curl -fsS --max-time 30 -o "$TMPU" "$DIVAN/api/tools/ulak?os=linux" && head -c 4 "$TMPU" | grep -q ELF; then
  echo "ulak indirilebiliyor"
else
  echo "UYARI: ulak indirilemedi; divan makinesinde DATA_DIR/tools/ulak-linux olmalı"
fi
rm -f "$TMPU"

adim "3/6 GitLab runner"
if ! command -v gitlab-runner >/dev/null; then
  curl -fsSL "https://packages.gitlab.com/install/repositories/runner/gitlab-runner/script.deb.sh" | bash
  apt-get install -y -q gitlab-runner
fi
if grep -q '^\s*token = ' /etc/gitlab-runner/config.toml 2>/dev/null; then
  echo "runner zaten kayıtlı; kayıt atlandı"
else
  if [ -z "${RUNNER_TOKEN:-}" ]; then
    read -r -s -p "Runner token'ı (glrt-…): " RUNNER_TOKEN; echo
  fi
  gitlab-runner register --non-interactive --url "$GITLAB/" --token "$RUNNER_TOKEN" \
    --executor shell --description "$(hostname)"
fi

adim "4/6 Runner ayarları"
sed -i 's/^concurrent = .*/concurrent = 1/' /etc/gitlab-runner/config.toml
# Ubuntu'nun .bash_logout'u shell executor'ı "prepare environment" hatasıyla düşürür
rm -f /home/gitlab-runner/.bash_logout
systemctl restart gitlab-runner

adim "5/6 Maven deposu"
if [ -n "$M2" ]; then
  install -d -o gitlab-runner -g gitlab-runner /home/gitlab-runner/.m2/repository
  cp -r "$M2"/. /home/gitlab-runner/.m2/repository/
  [ -f /home/gitlab-runner/.m2/settings.xml ] || echo '<settings><offline>true</offline></settings>' > /home/gitlab-runner/.m2/settings.xml
  chown -R gitlab-runner:gitlab-runner /home/gitlab-runner/.m2
  echo "çevrimdışı depo kopyalandı"
else
  echo "atlandı (--m2 verilmedi; Nexus ya da projenin ci/settings.xml'i kullanılıyor)"
fi

adim "6/6 Kontrol"
gitlab-runner verify 2>&1 | tail -n 3 || true
echo "java:  $(java -version 2>&1 | head -n 1)"
echo "maven: $(mvn -v 2>/dev/null | head -n 1)"
echo "xvfb:  $(command -v xvfb-run || echo YOK)"
if sudo -u gitlab-runner -i bash -c 'command -v opencode' >/dev/null 2>&1; then
  echo "opencode: var (gitlab-runner kullanıcısı görüyor)"
else
  echo "opencode: yok — bu VM fix işini koşmayacaksa sorun değil"
fi
echo
echo "Bitti. GitLab'da runner yeşil olmalı; etiketleri (linux, kişinin adı) runner sayfasından kontrol edin."
