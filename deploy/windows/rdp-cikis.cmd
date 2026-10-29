@echo off
rem SW-1.6: RDP bağlantısını kesince oturum kilitlenir ve TestFX olayları gitmez.
rem RDP'den çıkarken bu betiği YÖNETİCİ olarak çalıştırın: oturum konsola geri verilir,
rem masaüstü açık kalır, GitLab runner testleri koşmaya devam eder.
for /f "skip=1 tokens=3" %%s in ('query user %USERNAME%') do (
  %windir%\System32\tscon.exe %%s /dest:console
)
