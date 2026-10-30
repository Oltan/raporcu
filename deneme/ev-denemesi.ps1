# raporcu ev denemesi (Windows): GitLab olmadan tüm akışı gerçek JavaFX testleriyle koşturur.
#
#   ev-denemesi.cmd            (çift tıklama: 1 koşum)
#   powershell -ExecutionPolicy Bypass -File ev-denemesi.ps1 -Kosu 3
#
# Gerekenler: Java 21 (Temurin) ve Maven PATH içinde; isteğe bağlı ffmpeg (video).
# Testler masaüstünde gerçek pencere açar ve fareyi kullanır: koşum sırasında fareye/klavyeye dokunmayın.
param([int]$Kosu = 1, [int]$Port = 8080)
$ErrorActionPreference = "Stop"
$Kok = $PSScriptRoot
$Token = "ev-denemesi"

function Bul([string[]]$adaylar) { foreach ($p in $adaylar) { if (Test-Path $p) { return (Resolve-Path $p).Path } } return $null }
function Hata([string]$m) { Write-Host "HATA: $m" -ForegroundColor Red; Read-Host "Kapatmak için Enter"; exit 1 }

$Raporcu = Bul @("$Kok\raporcu.exe", "$Kok\..\raporcu.exe", "$Kok\..\target\x86_64-pc-windows-gnu\release\raporcu.exe", "$Kok\..\target\release\raporcu.exe")
$Stajci = Bul @("$Kok\tools\stajci-windows.exe", "$Kok\..\tools\stajci-windows.exe", "$Kok\..\target\x86_64-pc-windows-gnu\release\stajci.exe", "$Kok\..\target\release\stajci.exe")
$ProjeKaynak = Bul @("$Kok\ornek-test-projesi", "$Kok\..\ornek-test-projesi")
if (-not $Raporcu) { Hata "raporcu.exe bulunamadı (paketin kökünde olmalı)" }
if (-not $Stajci) { Hata "tools\stajci-windows.exe bulunamadı" }
if (-not $ProjeKaynak) { Hata "ornek-test-projesi klasörü bulunamadı" }
if (-not (Get-Command java -ErrorAction SilentlyContinue)) { Hata "java PATH içinde yok (Java 21 kurun)" }
if (-not (Get-Command mvn -ErrorAction SilentlyContinue)) { Hata "mvn PATH içinde yok (Maven kurun, bin klasörünü PATH'e ekleyin)" }
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) { Write-Host "Not: ffmpeg yok, video kaydı olmadan devam ediliyor" }

$Url = "http://127.0.0.1:$Port"
function Saglikli { try { Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 "$Url/api/health" | Out-Null; return $true } catch { return $false } }
if (Saglikli) { Hata "$Port portu dolu. Başka port: ev-denemesi.ps1 -Port 8090" }

$Is = Join-Path $Kok "ev-denemesi-veri"   # her şey burada; silmek yeterli
New-Item -ItemType Directory -Force -Path "$Is\veri\tools" | Out-Null
Copy-Item $Stajci "$Is\veri\tools\stajci-windows.exe" -Force
Write-Host "Çalışma klasörü: $Is"

# --- sunucu ---
$env:DATA_DIR = "$Is\veri"; $env:BIND_ADDR = "127.0.0.1:$Port"; $env:PUBLIC_URL = $Url
$env:CI_API_TOKEN = $Token; $env:LOG_STDOUT = "false"
$sunucu = Start-Process -FilePath $Raporcu -WorkingDirectory $Is -WindowStyle Hidden -PassThru
try {
  for ($i = 0; $i -lt 50 -and -not (Saglikli); $i++) { Start-Sleep -Milliseconds 200 }
  if (-not (Saglikli)) { Hata "sunucu açılmadı (log: $Is\veri\logs)" }
  Write-Host "raporcu açıldı: $Url"
  Start-Process "$Url/"

  # --- test projesi (kopyası üzerinde) ---
  $Proje = Join-Path $Is "ornek-test-projesi"
  if (Test-Path $Proje) { Remove-Item -Recurse -Force $Proje }
  Copy-Item -Recurse $ProjeKaynak $Proje
  Remove-Item -Recurse -Force "$Proje\target", "$Proje\.stajci" -ErrorAction SilentlyContinue
  $env:STAJ_SERVER = $Url; $env:CI_PROJECT_PATH = "ev/ornek-test-projesi"; $env:CI_COMMIT_REF_NAME = "main"
  Set-Location $Proje

  & $Stajci preflight
  if ($LASTEXITCODE -ne 0) { Hata "ön kontrol geçmedi" }
  for ($n = 1; $n -le $Kosu; $n++) {
    Write-Host ""; Write-Host "=== Koşum $n / $Kosu ===  (fareye dokunmayın)" -ForegroundColor Cyan
    Remove-Item ".stajci\run.json" -ErrorAction SilentlyContinue
    # koşum açılınca canlı ekranı tarayıcıda aç
    $izle = Start-Job -ArgumentList "$Proje\.stajci\run.json" -ScriptBlock {
      param($f)
      for ($i = 0; $i -lt 300; $i++) {
        if (Test-Path $f) { $u = (Get-Content $f -Raw | ConvertFrom-Json).report_url; if ($u) { Start-Process "$u/live"; return } }
        Start-Sleep -Milliseconds 200
      }
    }
    & $Stajci start
    Write-Host "(test işi çıkış kodu: $LASTEXITCODE - örnek projede bilerek kalan senaryolar var, 1 normaldir)"
    Remove-Job $izle -Force -ErrorAction SilentlyContinue
    & $Stajci upload
    & $Stajci finish
  }
  Write-Host ""
  Write-Host "Bitti. Dashboard: $Url/" -ForegroundColor Green
  Write-Host "Admin (paylaşım, triage) için: kapatın, $Is klasöründe `"$Raporcu`" set-admin-password, sonra betiği yeniden çalıştırın"
  Read-Host "Sunucu açık; kapatmak için Enter"
} finally {
  Set-Location $Kok
  Stop-Process -Id $sunucu.Id -ErrorAction SilentlyContinue
}
