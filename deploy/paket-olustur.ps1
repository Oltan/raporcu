# Çevrimdışı kurulum paketi: Windows ve Linux ikilileri, Java destek jar'ı, Maven çevrimdışı deposu,
# CI/ajan/skill şablonları ve KURULUM.md. Çıktı: dist\divan-kurulum.zip
# Gerekenler: Rust, JDK 21, Maven, internet (yalnızca paket yapılırken), Linux için WSL + Rust.
param(
  [string]$Cikti = (Join-Path $PSScriptRoot "..\dist"),
  [string]$MavenKoku = (Split-Path (Split-Path (Get-Command mvn).Source)),
  [switch]$LinuxYok
)
$ErrorActionPreference = "Stop"
$kok = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
New-Item -ItemType Directory -Force $Cikti | Out-Null
$Cikti = (Resolve-Path $Cikti).Path
$p = Join-Path $Cikti "divan-kurulum"
if (Test-Path $p) { Remove-Item $p -Recurse -Force }
foreach ($d in "windows", "linux", "sunucu", "java", "test-projesi\.opencode\agents", "test-projesi\.claude\skills", "ozel\agents", "ozel\skills\ornek-skill") {
  New-Item -ItemType Directory -Force (Join-Path $p $d) | Out-Null
}

Write-Host "== Windows ikilileri (statik C çalışma zamanı)"
$eski = $env:RUSTFLAGS
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --target x86_64-pc-windows-msvc --target-dir "$kok\target\paket" -p divan -p ulak
$env:RUSTFLAGS = $eski
if ($LASTEXITCODE -ne 0) { throw "Windows derlemesi başarısız" }
Copy-Item "$kok\target\paket\x86_64-pc-windows-msvc\release\divan.exe", "$kok\target\paket\x86_64-pc-windows-msvc\release\ulak.exe" "$p\windows\"

if (-not $LinuxYok) {
  Write-Host "== Linux ikilileri (WSL, musl)"
  $wslKok = (wsl -d Ubuntu -- wslpath -a ($kok -replace '\\', '/')).Trim()
  $wslP = (wsl -d Ubuntu -- wslpath -a ($p -replace '\\', '/')).Trim()
  wsl -d Ubuntu --cd $wslKok -- bash deploy/linux-derle.sh "$wslP/linux"
  if ($LASTEXITCODE -ne 0) { throw "Linux derlemesi başarısız" }
}

Write-Host "== Sunucu dosyaları"
Copy-Item "$kok\.env.example" "$p\sunucu\env.ornek"
Copy-Item "$kok\deploy\linux\divan.service", "$kok\deploy\windows\kurulum.ps1", "$kok\deploy\windows\rdp-cikis.cmd", "$kok\ci\gitlab\runner\config.toml.example" "$p\sunucu\"
Copy-Item "$kok\deploy\projeye-ekle.ps1", "$kok\deploy\projeye-ekle.sh", "$kok\docs\KURULUM.md" $p
Copy-Item "$kok\docs\KOD_AJANI_VLLM.md", "$kok\docs\LLM_JAVA_ENTEGRASYON.md", "$kok\docs\JIRA_DOORS_AYARLARI.md", "$kok\docs\GITLAB_CI_KURULUMU.md" "$p\sunucu\"

Write-Host "== Test projesi şablonu"
$t = "$p\test-projesi"
Copy-Item "$kok\ci\gitlab\template\.gitlab-ci.yml" $t
Copy-Item "$kok\ci\opencode\template\opencode.json", "$kok\ci\opencode\template\AGENTS.md" $t
Copy-Item "$kok\ci\opencode\template\.opencode\agents\fix.md" "$t\.opencode\agents\"
foreach ($s in "cucumber-adim", "hata-siniflandirma", "testfx-secici") { Copy-Item "$kok\ci\skills\$s" "$t\.claude\skills\" -Recurse }
Copy-Item "$kok\ci\opencode\template\.opencode\agents\fix.md" "$p\ozel\agents\ORNEK-ajan.md"
Copy-Item "$kok\ci\skills\testfx-secici\SKILL.md" "$p\ozel\skills\ornek-skill\SKILL.md"
Copy-Item "$kok\ci\java-destek\pom-eklentisi.xml" "$p\java\"

Write-Host "== Java destek jar'ı ve çevrimdışı Maven deposu"
$is = Join-Path $env:TEMP "divan-java-paket"
if (Test-Path $is) { Remove-Item $is -Recurse -Force }
$kod = "$is\destek\src\main\java\divan\destek"
New-Item -ItemType Directory -Force $kod | Out-Null
Copy-Item "$kok\ci\java-destek\pom.xml" "$is\destek\"
foreach ($f in "$kok\ornek-test-projesi\src\test\java\com\ornek\test\CanliOlayEklentisi.java", "$kok\ornek-test-projesi\src\test\java\com\ornek\test\VideoKaydi.java", "$kok\ci\java-sablon\DivanKanit.java") {
  $metin = [IO.File]::ReadAllText($f).Replace("com.ornek.test", "divan.destek")
  [IO.File]::WriteAllText("$kod\$(Split-Path $f -Leaf)", $metin)
}
$m2 = "$p\java\m2"
$mvn = Join-Path $MavenKoku "bin\mvn.cmd"
& $mvn -B -q -f "$is\destek\pom.xml" "-Dmaven.repo.local=$m2" install
if ($LASTEXITCODE -ne 0) { throw "destek jar'ı derlenemedi" }
Copy-Item "$is\destek\target\divan-test-destek-1.0.0.jar" "$p\java\"
# Örnek proje iki platformun bağımlılıklarını ve surefire sağlayıcılarını depoya indirir; senaryo seçilmez.
Copy-Item "$kok\ornek-test-projesi" "$is\ornek" -Recurse -Exclude target
foreach ($os in "win", "linux") {
  & $mvn -B -q -f "$is\ornek\pom.xml" "-Dmaven.repo.local=$m2" "-Djavafx.platform=$os" "-Dcucumber.filter.tags=@paket-yok" clean test
  if ($LASTEXITCODE -ne 0) { throw "bağımlılıklar indirilemedi ($os)" }
}
# Ubuntu paketindeki Maven eski eklenti sürümleri ister; onlar da depoya insin
if (-not $LinuxYok) {
  $wslM2 = (wsl -d Ubuntu -- wslpath -a ($m2 -replace '\\', '/')).Trim()
  $wslOrnek = (wsl -d Ubuntu -- wslpath -a (("$is\ornek") -replace '\\', '/')).Trim()
  wsl -d Ubuntu --cd $wslOrnek -- mvn -B -q "-Dmaven.repo.local=$wslM2" "-Dcucumber.filter.tags=@paket-yok" clean test
  if ($LASTEXITCODE -ne 0) { throw "Ubuntu Maven eklentileri indirilemedi" }
}
# Depo kimliği kayıtları silinir: başka settings.xml ile çevrimdışı çalışsın
Get-ChildItem $m2 -Recurse -File -Force | Where-Object { $_.Name -in "_remote.repositories", "resolver-status.properties" -or $_.Name -like "*.lastUpdated" } | Remove-Item -Force

Write-Host "== Maven"
Copy-Item $MavenKoku "$p\java\maven" -Recurse

Write-Host "== SHA256 ve zip"
Get-ChildItem $p -File -Recurse | Where-Object { $_.FullName -notlike "*\m2\*" -and $_.FullName -notlike "*\maven\*" } |
  ForEach-Object { "{0}  {1}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower(), $_.FullName.Substring($p.Length + 1) } |
  Set-Content "$p\SHA256.txt" -Encoding utf8NoBOM
$zip = Join-Path $Cikti "divan-kurulum.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path $p -DestinationPath $zip -CompressionLevel Optimal
Write-Host ("Hazır: {0} ({1:N0} MB)" -f $zip, ((Get-Item $zip).Length / 1MB))
