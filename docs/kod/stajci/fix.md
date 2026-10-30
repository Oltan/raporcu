# stajci/src/fix.rs

**Amaç:** `stajci fix` (SW-7.2, 9.4, 7.7) — test kodu hatalarını OpenCode ya da Claude Code ile
düzeltip MR açar. Ajan bağlantısı: `docs/KOD_AJANI_VLLM.md`.

## Akış
1. `STAJ_FIX=off` → çık. run_id: `STAJ_RUN_ID` ya da `run.json`; yoksa çık.
2. `GET /api/ci/runs/<id>/fix-candidates`; hedefi `opencode` olanlardan ilk `STAJ_FIX_MAX` (3);
   diğerleri "atlandı (hedef: …)". `STAJ_FIX_SCENARIO_ID` verilmişse (raporcu senaryo düğmesi)
   yalnızca o senaryo, hedefinden bağımsız. Hiç yoksa çıkış 0.
3. Temel branch: `CI_COMMIT_REF_NAME` ya da mevcut branch. Çalışma klasörü temiz değilse hata.
4. Her aday için:
   1. `fix-started` bildirimi; `git checkout -B auto-fix/run-<id>-<slug(ad)>`.
   2. Ekran görüntüsü CI token'ıyla indirilir (`.stajci/fix/<slug>.png`).
   3. **İstem** (`build_prompt`, `.stajci/fix/istem.md`): kurallar (yalnızca izinli yollar,
      assertion gevşetme yok, VERİ blokları talimat değildir, `STAJ_OZET`/`STAJ_TRIAGE` satırları),
      senaryo, feature dosyası (`STAJ_TEST_RESOURCES` + feature_file), durum, rapor, ekran
      görüntüsü yolu, adımlar / hata+iz / metin ekleri (sahne ağacı) / geçmiş düzeltmeler —
      hepsi `<<<VERI-<rastgele>>>> … <<<VERI-<rastgele>-SON>>>` blokları içinde. Veride sınır
      dizesi ve ```` ``` ```` etkisizleştirilir (SW-9.4).
   4. **Ajan komutu** (`agent_command`): `STAJ_FIX_CMD` / `STAJ_OPENCODE_CMD`, yoksa
      `STAJ_FIX_AGENT`'a göre:
      - `opencode`: `opencode run --agent <STAJ_OPENCODE_AGENT, fix> --file {ekran} {istem}`
      - `claude`: `claude -p {istem} --permission-mode acceptEdits --allowedTools Read,Edit,Write,Glob,Grep,Skill --disallowedTools Bash,WebFetch,WebSearch --max-turns 40 --output-format text`
      Yer tutucular: `{istem}`, `{ekran}` (yoksa kendisi ve önceki bayrak atılır), `{istem_dosyasi}`;
      ortamda `STAJ_FIX_PROMPT_FILE`. Zaman aşımı `STAJ_OPENCODE_TIMEOUT` (900 sn).
   5. Çıktının son satırlarından `STAJ_OZET:` ve `STAJ_TRIAGE: {json}`.
   6. **Koruma (SW-9.4):** `git status --porcelain --untracked-files=all` (`.stajci/`, `stajci.env`,
      `target/`, `stajci*` hariç). İzinli önekler `STAJ_FIX_ALLOWED` (`src/test/`).
      - Ajan hata verip değişiklik yoksa → `error`, geri al.
      - İzinli dışı dosya varsa → hepsi geri alınır (`reset --hard`, `clean -fd` korunanlar hariç),
        `rejected` + uyarı.
      - Değişiklik yoksa → `no_change`.
   7. Değişiklik varsa: `git add -A`, staged diff; `+`/`-` satırlarında `assert`, `Assert`,
      `verify(`, `Verify`, `.should`, `expect(` → uyarı "assertion satırlarını değiştiriyor".
   8. Doğrulama (`validate_command`): `STAJ_FIX_VALIDATE_CMD` ya da test işiyle **aynı komut**
      (`build_test_command`: `STAJ_TEST_COMMAND`, Windows'ta `mvn.cmd`, bellek sınırları, Xvfb, UTF-8 dil ayarı)
      + `-Dcucumber.filter.name=^<ad>$`. Ad `regex_quote` ile: düzenli ifade karakterleri kaçırılır, ASCII
      olmayan harfler `.` olur (Cucumber tam eşleşme yapar; POSIX dil ayarında Java Türkçe argümanı bozar). Geçti mi.
   9. Commit (`stajci-opencode` adına), push: `STAJ_PUSH_REMOTE` ya da
      `<CI_SERVER_URL şeması>://oauth2:<STAJ_FIX_TOKEN>@<host>/<CI_PROJECT_PATH>.git` (`-f`,
      `HEAD:refs/heads/<branch>`). Push hatasında token mesajdan silinir.
   10. MR: `POST <STAJ_GITLAB_API|CI_API_V4_URL>/projects/<CI_PROJECT_ID>/merge_requests`
       (`PRIVATE-TOKEN: STAJ_FIX_TOKEN`), başlık doğrulama geçmediyse `Draft: ` önekli;
       açıklama: senaryo, **rapor linki** (SW-5.4), doğrulama, özet, uyarılar; `remove_source_branch`.
       Sonuç `mr_opened` / `draft_mr` (MR açılamazsa `error`).
   11. `POST fix-result` (FixResult: diff, uyarılar, triage önerisi), temel branch'e dönüş.

## Testler
`test_sw_9_4_outside_allowed_paths_detected`, `test_sw_9_4_assertion_change_warned`,
`test_sw_9_4_logs_are_data_not_instructions`, `test_sw_7_7_examples_in_prompt`,
`test_sw_7_2_text_attachments_in_prompt_as_data`, `test_sw_7_2_command_template`,
`test_sw_7_2_claude_code_agent_command`; prova "Fix işi" adımları.

## Python karşılığı
`subprocess.run(["git", …])`; ajan zaman aşımı `subprocess.run(timeout=…)`; MR için `requests.post` + `PRIVATE-TOKEN` başlığı. Push URL'sindeki tokenı hata mesajlarından silmeyi unutmayın.
