# SW-1.6 / SW-10.1: Windows test sanal makinesi hazırlığı. YÖNETİCİ PowerShell'de çalıştırın.
# Parametreler: test kullanıcısı ve runner klasörü
param(
  [string]$Kullanici = "testrunner",
  [string]$RunnerDir = "C:\GitLab-Runner",
  [string]$StajDir = "C:\raporcu"
)

# 1) Uyku, ekran kapanması ve hazırda bekleme kapalı
powercfg /change standby-timeout-ac 0
powercfg /change monitor-timeout-ac 0
powercfg /change hibernate-timeout-ac 0
powercfg /hibernate off

# 2) Ekran koruyucu ve kilit kapalı (kullanıcı ve makine ilkesi)
reg add "HKCU\Control Panel\Desktop" /v ScreenSaveActive /t REG_SZ /d 0 /f
reg add "HKCU\Control Panel\Desktop" /v ScreenSaverIsSecure /t REG_SZ /d 0 /f
reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows\Personalization" /v NoLockScreen /t REG_DWORD /d 1 /f
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System" /v InactivityTimeoutSecs /t REG_DWORD /d 0 /f

# 3) Otomatik oturum açma: Sysinternals Autologon önerilir (şifreyi LSA'da şifreli saklar):
#      Autologon64.exe $Kullanici $env:COMPUTERNAME <şifre> /accepteula
Write-Host "Otomatik oturum açma için: Autologon64.exe $Kullanici $env:COMPUTERNAME <sifre> /accepteula"

# 4) GitLab runner hizmet olarak DEĞİL, oturumda başlar: oturum açılınca çalışan zamanlanmış görev
$runner = Join-Path $RunnerDir "gitlab-runner.exe"
schtasks /create /tn "GitLab Runner (masaustu)" /tr "`"$runner`" run --working-directory `"$RunnerDir`" --config `"$RunnerDir\config.toml`"" /sc onlogon /ru $Kullanici /rl highest /f

# 5) raporcu sunucusu: makine açılınca (oturum gerektirmez) SYSTEM olarak
$server = Join-Path $StajDir "raporcu.exe"
schtasks /create /tn "raporcu sunucusu" /tr "cmd /c cd /d `"$StajDir`" && `"$server`"" /sc onstart /ru SYSTEM /rl highest /f

Write-Host "Tamam. Ekran çözünürlüğünü sabitleyin (Ayarlar > Ekran, ör. 1920x1080) ve makineyi yeniden başlatın."
