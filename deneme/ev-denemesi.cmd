@echo off
rem divan ev denemesi: cift tiklayin (1 kosum). Daha fazla kosum: ev-denemesi.ps1 -Kosu 3
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0ev-denemesi.ps1" %*
