# Yeni test projesi ekleme (SW-10.4)

Bir Cucumber + TestFX (JavaFX) test projesini divan'a bağlamak için. Hazır örnek:
`ornek-test-projesi/`. Küçük bir yerel modelle adım adım yapılacaksa:
`docs/LLM_JAVA_ENTEGRASYON.md`.

**1. Allure adaptörü (`pom.xml`)** — `ornek-test-projesi/pom.xml` içinden kopyalayın:
`io.qameta.allure:allure-cucumber7-jvm` bağımlılığı; surefire'da `forkCount=1`,
`<argLine>-Xmx${test.xmx} -Djava.awt.headless=false</argLine>` ve `rerunFailingTestsCount`.
Monocle/headless ayarı olmamalı.

**2. Cucumber eklentisi** — `src/test/resources/junit-platform.properties`:

    cucumber.plugin=pretty, io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm

**3. Hook'lar** — `ornek-test-projesi/src/test/java/com/ornek/test/Hooks.java` ve `VideoKaydi.java`:
senaryo kalınca `scenario.attach(png, "image/png", ...)` ile ekran görüntüsü ve video eklenir.

**4. Tag'ler** — DOORS numarası `@DOORS-1234`, bilinen Jira bug'ı `@JIRA-ABC-12`. Koşulacak
senaryolar proje kökündeki `features.txt` dosyasında (her satır bir tag ifadesi).

**5. CI** — `.gitlab-ci.yml`:

```yaml
include:
  - project: 'ekip/divan'
    ref: main
    file: '/ci/gitlab/template/.gitlab-ci.yml'
variables:
  ULAK_OS: linux      # ya da windows
```

Tam örnek: `ornek-test-projesi/.gitlab-ci.yml`. divan deposu şirket GitLab'ında değilse
`ci/gitlab/template/.gitlab-ci.yml` dosyasını include etmek yerine olduğu gibi kopyalayın.

**6. GitLab ayarları** — CI değişkenleri `CI_API_TOKEN` (masked), self-healing için
`ULAK_FIX_TOKEN`; webhook (Pipeline + Merge request events); gece schedule'ı 02:00.
Ayrıntı: `docs/GITLAB_CI_KURULUMU.md` bölüm 4–5.

**7. OpenCode** — `ci/opencode/template/` içindeki `opencode.json`, `.opencode/agents/fix.md` ve
`AGENTS.md`'yi projeye kopyalayın; `AGENTS.md`'yi projenize göre doldurun. Skill'ler:
`ci/skills/*` → projede `.claude/skills/` (OpenCode ve Claude Code ikisi de okur). Ayrıntı:
`docs/KOD_AJANI_VLLM.md`.

**8. Deneyin** — branch'e push edin; pipeline'da `ulak preflight` eksikleri açık mesajla söyler.
İlk yükleme bittiğinde proje dashboard'da kendiliğinden görünür; rapor linki report işinin
çıktısında ve pipeline'ın environment linkindedir.
