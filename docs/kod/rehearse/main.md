# rehearse/src/main.rs

**Amaç:** SW-10.2 — GitLab CI akışını yerelde, GitLab olmadan uçtan uca benzetir:
`cargo run -p rehearse` (Windows'ta Wine ile de). Her yeni CI adımı buraya bir kontrol olarak
eklenir. Çıkış 0 = hepsi geçti.

## Sahte süreçler (aynı ikilinin alt komutları)
| Alt komut | Neyin yerine | Davranış |
|-----------|--------------|----------|
| `fake-tests` | `mvn test` | ulak'ın verdiği `-Dallure.results.directory`, `-Dulak.canli.dosya`, `-Dtest.xmx` argümanlarını okur; `--sample` klasöründeki Allure dosyalarını **senaryo senaryo** `--delay-ms` aralıkla yazar (önce ekler, sonra result); her senaryo için Cucumber olay satırları (`senaryo_basladi`, `adim_basladi/bitti`, `senaryo_bitti`) yazar; `--env-out` ile aldığı argüman/ortamı kaydeder; `--oom` bellek hatası satırı basar; kalan senaryo varsa çıkış 1 |
| `fake-opencode` | kod ajanı | istem dosyasını okur, `src/test/` altında bir dosyayı değiştirir (ya da `ULAK_FAKE_AGENT` ile izinli dışına yazar / assertion değiştirir), `ULAK_OZET` / `ULAK_TRIAGE` satırları basar |
| `fake-validate` | doğrulama koşusu | `ULAK_FAKE_VALIDATE=fail` ise 1 |
| `fake_gitlab()` | GitLab API | yerel HTTP sunucusu; MR açma isteklerini kaydeder |

## Adımlar
1. **CI şablonu** (`template_checks`): `ci/gitlab/template/.gitlab-ci.yml` Linux/Windows için test,
   report, fix işlerini, etiketleri, `ulak start/upload/finish` çağrılarını, `resource_group`,
   environment linki, artifact'ları içerir (SW-1.1, 1.2, 1.7, 2.6, 5.4).
2. **Dağıtım:** sunucu başlar, `GET /api/tools/ulak?os=…` ile ulak indirilir (SW-10.9).
3. **Ön kontrol:** örnek proje geçer, Allure'suz kopya düşer (SW-1.5).
4. **Test işi:** `ulak start` + `fake-tests`; testler sürerken canlı API yoklanır: ilerleme x/7
   (SW-3.1), şu an koşan senaryo/adım (SW-3.4), senaryo bittikten ≤5 sn sonra görünme (SW-3.2),
   bellek argümanları (SW-1.7), Xvfb sarmalama (SW-1.2), rapor linki (SW-5.4), run_id'li loglar
   (SW-10.7), kalan senaryoda çıkış 1.
5. **Report işi:** `ulak upload` tekrarları saymaz (SW-3.3), `finish` (SW-3.5), rerun (SW-4.3/4.9).
6. **OOM** → çıkış 3 (SW-1.7); **Windows hizmet oturumu** → çıkış 4 (SW-1.6).
7. **Sunucu kapalıyken:** upload kuyruğa alır, sunucu açılınca `upload --resume` gönderir (SW-2.6).
8. **Fix işi** (`extra_steps`): yerel bare git deposu + sahte GitLab + `fake-opencode`; MR açılır,
   rapor linki MR'da, Draft (doğrulama kalınca), raporda düzeltme kutusu, izinli dışı değişiklik
   reddedilir, assertion uyarısı, VERİ blokları (SW-7.2, 7.3, 4.4, 9.4, 5.4).

## Python karşılığı
Aynı senaryo `pytest` ile yazılabilir ama gerçek süreçlerle kalmalıdır (prova sunucu ve aracı
dışarıdan sınar; dil değişse de bu adımlar aynı kalır — taşıma sonrası kabul testi olarak kullanın).
