# Pilot projede manuel kontrol listesi (GitLab, Windows, dış servisler)

Bu ortamda (Linux, GitLab'sız) doğrulanamayan kabul kriterleri. Her madde bitince
`GEREKSINIMLER_1.md` içinde ilgili SW durumu `MEVCUT` yapılır.

## Ubuntu runner

- [ ] SW-1.1 Pipeline 3 iş çalıştırır; test işi düşünce report ve fix yine çalışır.
- [ ] SW-1.2 Kimse bağlı değilken gece koşumu geçer; ekran görüntüsü/video gerçek uygulamayı gösterir;
      aynı makinede iki iş aynı anda koşunca ekranlar karışmaz (`concurrent` geçici 2 yapılarak).
- [ ] SW-1.3 Schedule 02:00'de başlar, sabah rapor hazır.
- [ ] SW-1.4 `include` satırı ile çalışır; şablon değişince ertesi koşum yeni sürümü kullanır.
- [ ] SW-1.5 Allure bağımlılığı silinmiş bir branch'te iş "Allure adaptörü eksik" mesajıyla düşer.
- [ ] SW-1.7 `ULAK_TEST_XMX=64m` ile iş "BELLEK SINIRI AŞILDI" mesajıyla düşer; normal gece OOM yok.
- [ ] SW-3.6 Pipeline başlayınca dashboard'da "kuyrukta / çalışıyor".
- [ ] SW-5.4 Rapor linki report işi çıktısında ve pipeline environment linkinde.
- [ ] SW-10.9 Sunucu güncellenince ertesi koşum yeni ulak'ı indirir.

## Windows runner

- [ ] SW-1.6 Makine yeniden başlayınca otomatik oturum açılır, runner görev olarak başlar;
      kimse bağlı değilken gece koşumu geçer; RDP'den `rdp-cikis.cmd` ile çıkınca koşum devam eder.
- [ ] Yukarıdaki Ubuntu maddeleri Windows'ta da (SW-1.1, 1.5, 1.7, 2.6, 3.2, 9.4, 10.2, 10.9):
      `cargo run -p rehearse` ve `cargo run -p e2esmoke` Windows'ta geçer.

## Dış servisler

- [ ] SW-2.7 Gerçek test projesinden `allure-results` örnekleri `testdata/allure/<proje>/` altına
      kondu (geçen, kalan, broken, rerun, ekli); testler bunlarla da geçer.
- [ ] SW-5.3 Gerçek SMTP relay ile pipeline başına tek e-posta.
- [ ] SW-6.2 Gerçek Jira'da açık bug'ı olan senaryo yükleme sonrası bağlanır (E testi).
- [ ] SW-6.4 DXL betiği gerçek DOORS modülünde çalışır; öznitelik adları modüle göre ayarlandı.
- [ ] SW-7.2 Gerçek OpenCode + vLLM + TestFX MCP + GitLab MR ile bir test kodu hatası düzeltilir.
- [ ] SW-7.4 Rapordaki "OpenCode ile düzelt" düğmesi yalnızca fix işini tetikler, sonuç aynı rapora yazılır.
- [ ] SW-7.6 `opencode.json` ve `fix` ajanı kurulu OpenCode sürümüyle doğrulandı.

## Kurulum

- [ ] SW-10.1 Rehberle sıfırdan Ubuntu ve Windows kurulumu yapıldı.
- [ ] SW-10.3 Repo şirket GitLab'ında (push kapalıysa `git bundle create divan.bundle --all`).
- [ ] SW-10.4 Başka bir ekip rehberle kendi projesini bağladı.
