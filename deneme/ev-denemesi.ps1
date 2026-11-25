# divan ev denemesi (Windows): GitLab olmadan tüm akışı gerçek JavaFX testleriyle koşturur.
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

$Divan = Bul @("$Kok\divan.exe", "$Kok\..\divan.exe", "$Kok\..\target\x86_64-pc-windows-gnu\release\divan.exe", "$Kok\..\target\release\divan.exe")
$Ulak = Bul @("$Kok\tools\ulak-windows.exe", "$Kok\..\tools\ulak-windows.exe", "$Kok\..\target\x86_64-pc-windows-gnu\release\ulak.exe", "$Kok\..\target\release\ulak.exe")
$ProjeKaynak = Bul @("$Kok\ornek-test-projesi", "$Kok\..\ornek-test-projesi")
if (-not $Divan) { Hata "divan.exe bulunamadı (paketin kökünde olmalı)" }
if (-not $Ulak) { Hata "tools\ulak-windows.exe bulunamadı" }
if (-not $ProjeKaynak) { Hata "ornek-test-projesi klasörü bulunamadı" }
if (-not (Get-Command java -ErrorAction SilentlyContinue)) { Hata "java PATH içinde yok (Java 21 kurun)" }
if (-not (Get-Command mvn -ErrorAction SilentlyContinue)) { Hata "mvn PATH içinde yok (Maven kurun, bin klasörünü PATH'e ekleyin)" }
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) { Write-Host "Not: ffmpeg yok, video kaydı olmadan devam ediliyor" }

$Url = "http://127.0.0.1:$Port"
function Saglikli { try { Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 "$Url/api/health" | Out-Null; return $true } catch { return $false } }
if (Saglikli) { Hata "$Port portu dolu. Başka port: ev-denemesi.ps1 -Port 8090" }

$Is = Join-Path $Kok "ev-denemesi-veri"   # her şey burada; silmek yeterli
New-Item -ItemType Directory -Force -Path "$Is\veri\tools" | Out-Null
Copy-Item $Ulak "$Is\veri\tools\ulak-windows.exe" -Force
Write-Host "Çalışma klasörü: $Is"

# --- sunucu ---
$env:DATA_DIR = "$Is\veri"; $env:BIND_ADDR = "127.0.0.1:$Port"; $env:PUBLIC_URL = $Url
$env:CI_API_TOKEN = $Token; $env:LOG_STDOUT = "false"
$sunucu = Start-Process -FilePath $Divan -WorkingDirectory $Is -WindowStyle Hidden -PassThru
try {
  for ($i = 0; $i -lt 50 -and -not (Saglikli); $i++) { Start-Sleep -Milliseconds 200 }
  if (-not (Saglikli)) { Hata "sunucu açılmadı (log: $Is\veri\logs)" }
  Write-Host "divan açıldı: $Url"
  Start-Process "$Url/"

  # --- test projesi (kopyası üzerinde) ---
  $Proje = Join-Path $Is "ornek-test-projesi"
  if (Test-Path $Proje) { Remove-Item -Recurse -Force $Proje }
  Copy-Item -Recurse $ProjeKaynak $Proje
  Remove-Item -Recurse -Force "$Proje\target", "$Proje\.ulak" -ErrorAction SilentlyContinue
  $env:ULAK_SERVER = $Url; $env:CI_PROJECT_PATH = "ev/ornek-test-projesi"; $env:CI_COMMIT_REF_NAME = "main"
  Set-Location $Proje

  & $Ulak preflight
  if ($LASTEXITCODE -ne 0) { Hata "ön kontrol geçmedi" }
  for ($n = 1; $n -le $Kosu; $n++) {
    Write-Host ""; Write-Host "=== Koşum $n / $Kosu ===  (fareye dokunmayın)" -ForegroundColor Cyan
    Remove-Item ".ulak\run.json" -ErrorAction SilentlyContinue
    # koşum açılınca canlı ekranı tarayıcıda aç
    $izle = Start-Job -ArgumentList "$Proje\.ulak\run.json" -ScriptBlock {
      param($f)
      for ($i = 0; $i -lt 300; $i++) {
        if (Test-Path $f) { $u = (Get-Content $f -Raw | ConvertFrom-Json).report_url; if ($u) { Start-Process "$u/live"; return } }
        Start-Sleep -Milliseconds 200
      }
    }
    & $Ulak start
    Write-Host "(test işi çıkış kodu: $LASTEXITCODE - örnek projede bilerek kalan senaryolar var, 1 normaldir)"
    Remove-Job $izle -Force -ErrorAction SilentlyContinue
    & $Ulak upload
    & $Ulak finish
  }
  Write-Host ""
  Write-Host "Bitti. Dashboard: $Url/" -ForegroundColor Green
  Write-Host "Admin (paylaşım, triage) için: kapatın, $Is klasöründe `"$Divan`" set-admin-password, sonra betiği yeniden çalıştırın"
  Read-Host "Sunucu açık; kapatmak için Enter"
} finally {
  Set-Location $Kok
  Stop-Process -Id $sunucu.Id -ErrorAction SilentlyContinue
}
