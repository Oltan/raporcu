#!/usr/bin/env bash
# Paketteki CI, ajan ve skill dosyalarını bir test projesine kopyalar. Var olan dosyaya dokunmaz;
# ezmek için üçüncü argüman "ustune". Kendi ajanlarınız ozel/agents/, skill'leriniz ozel/skills/ altında.
# Kullanım: ./projeye-ekle.sh /isler/test-projesi linux
set -euo pipefail

PROJE=$1
OS=${2:-linux}
USTUNE=${3:-}
KOK=$(cd "$(dirname "$0")" && pwd)
[ -f "$PROJE/pom.xml" ] || { echo "pom.xml yok: $PROJE bir Maven projesi olmalı" >&2; exit 1; }

kopyala() {
  if [ -e "$2" ] && [ "$USTUNE" != "ustune" ]; then echo "ATLANDI (var): $2"; return; fi
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2"
  echo "KOPYALANDI: $2"
}

(cd "$KOK/test-projesi" && find . -type f) | while read -r f; do
  kopyala "$KOK/test-projesi/$f" "$PROJE/$f"
done
for f in "$KOK"/ozel/agents/*.md; do
  [ -e "$f" ] && [ "$(basename "$f")" != "ORNEK-ajan.md" ] && kopyala "$f" "$PROJE/.opencode/agents/$(basename "$f")"
done
(cd "$KOK/ozel/skills" && find . -type f -not -path './ornek-skill/*') | while read -r f; do
  kopyala "$KOK/ozel/skills/$f" "$PROJE/.claude/skills/$f"
done

sed -i "s/^  ULAK_OS: \".*\"/  ULAK_OS: \"$OS\"/" "$PROJE/.gitlab-ci.yml"
echo "Tamam. Sıradaki: java/pom-eklentisi.xml içeriğini pom.xml'e ekleyin, sonra 'ulak preflight' çalıştırın."
