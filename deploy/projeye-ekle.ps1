# Paketteki CI, ajan ve skill dosyalarını bir test projesine kopyalar. Var olan dosyaya dokunmaz;
# ezmek için -Ustune. Kendi ajanlarınız ozel\agents\, skill'leriniz ozel\skills\ altında durur.
# Kullanım: .\projeye-ekle.ps1 -Proje C:\isler\test-projesi -Os windows
param(
  [Parameter(Mandatory = $true)][string]$Proje,
  [ValidateSet("windows", "linux")][string]$Os = "windows",
  [string]$PaketKoku = $PSScriptRoot,
  [switch]$Ustune
)
$ErrorActionPreference = "Stop"
if (-not (Test-Path (Join-Path $Proje "pom.xml"))) { throw "pom.xml yok: $Proje bir Maven projesi olmalı" }

function Kopyala($kaynak, $hedef) {
  if ((Test-Path $hedef) -and -not $Ustune) { Write-Host "ATLANDI (var): $hedef"; return }
  New-Item -ItemType Directory -Force (Split-Path $hedef) | Out-Null
  Copy-Item $kaynak $hedef -Force
  Write-Host "KOPYALANDI: $hedef"
}

$sablon = Join-Path $PaketKoku "test-projesi"
Get-ChildItem $sablon -Recurse -File -Force | ForEach-Object {
  Kopyala $_.FullName (Join-Path $Proje $_.FullName.Substring($sablon.Length + 1))
}
$ozel = Join-Path $PaketKoku "ozel"
Get-ChildItem (Join-Path $ozel "agents") -Filter *.md -File -ErrorAction SilentlyContinue | Where-Object Name -ne "ORNEK-ajan.md" | ForEach-Object {
  Kopyala $_.FullName (Join-Path $Proje ".opencode\agents\$($_.Name)")
}
Get-ChildItem (Join-Path $ozel "skills") -Filter SKILL.md -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.Directory.Name -ne "ornek-skill" } | ForEach-Object {
  $kok = Join-Path $ozel "skills"
  Get-ChildItem $_.Directory -Recurse -File | ForEach-Object {
    Kopyala $_.FullName (Join-Path $Proje ".claude\skills\$($_.FullName.Substring($kok.Length + 1))")
  }
}

$ci = Join-Path $Proje ".gitlab-ci.yml"
$metin = [IO.File]::ReadAllText($ci)
[IO.File]::WriteAllText($ci, ($metin -replace '(?m)^  ULAK_OS: ".*"', "  ULAK_OS: `"$Os`""))
Write-Host "Tamam. Sıradaki: java\pom-eklentisi.xml içeriğini pom.xml'e ekleyin, sonra 'ulak preflight' çalıştırın."
