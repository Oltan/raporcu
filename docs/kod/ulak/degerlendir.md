# ulak/src/degerlendir.rs

**Amaç:** `ulak degerlendir` (SW-4.5) — kalan senaryoların nedenini kod ajanına sorar. Ajan kodu
okur, değiştirmez; branch, doğrulama koşusu ve MR yoktur. CI'da fix işinde `ulak fix`ten önce koşar:
geçerli karar değişirse `fix-candidates` hedefleri de değişir. Ayrıntı: `docs/KOD_AJANI_VLLM.md`.

## Ne zaman çalışır (`enabled`)
- `ULAK_DEGERLENDIR=off` → çalışmaz.
- `ULAK_FIX_ONLY=true` (rapordaki düğme) → çalışmaz, koşum yeniden değerlendirilmez.
- `ULAK_DEGERLENDIR=on` → çalışır (`ULAK_FIX=off` olsa da: yalnızca değerlendirme).
- Diğer: `ULAK_FIX=off` değilse çalışır.

## Akış
1. Koşum: `fix::current_run`. Adaylar: `GET fix-candidates`; hedefi `atla` (flaky, `@sample-fail`) ve
   `acik-mr` olanlar atlanır (`evaluable`). Üst sınır `ULAK_DEGERLENDIR_MAX` (20); fazlası kural
   kararıyla kalır. Çalışma klasörü temiz değilse değerlendirme yapılmaz.
2. `.ulak/degerlendir/` silinip yeniden açılır: eski ajan logları yeni kararı etkilemesin.
3. Her senaryo için ekran görüntüsü indirilir, istem (`evaluation_prompt`) kurulur: kurallar + `fix::push_failure`
   (test çıktısı VERİ bloklarında, SW-9.4). `.ulak/degerlendir/istem.md`.
4. Ajan komutu (`evaluation_command`): `ULAK_DEGERLENDIR_CMD`, yoksa `ULAK_FIX_AGENT`'a göre:
   - `opencode`: `<ULAK_OPENCODE_BIN> run --agent <ULAK_DEGERLENDIR_AGENT, degerlendir> [--model <ULAK_OPENCODE_MODEL>] {istem} --file {ekran}`
   - `claude`: `claude -p {istem} --allowedTools Read,Glob,Grep,Skill --disallowedTools Edit,Write,Bash,WebFetch,WebSearch --max-turns 15 --output-format text`
   Zaman aşımı `ULAK_DEGERLENDIR_TIMEOUT` (300 sn). Çıktı `.ulak/degerlendir/<senaryo>-ajan.log`.
5. **Koruma:** ajandan sonra `fix::changed_files`; değişiklik varsa `fix::restore` ile geri alınır,
   karar kullanılmaz.
6. `ULAK_TRIAGE` satırı okunur. Karar yoksa uyarı + bir deneme daha (reddedilen araç oturumu kapatabilir).
   Zaman aşımında yeniden denenmez ve değerlendirme durur: kota dolunca OpenCode hata vermeden bekler,
   model yanıt vermiyorsa kalan senaryolar da bekleyecektir; onlar kural kararıyla kalır. Ajanın
   başlattığı alt süreçler de sonlandırılır (`fix.md`, süreç ağacı).
7. `POST /api/ci/runs/{id}/triage` (`AgentTriage`).

Hatalar uyarıdır, çıkış kodu her zaman 0: değerlendirme kurulamazsa `ulak fix` yine çalışır ve kural
kararları geçerli kalır.

## Testler
`test_sw_4_5_evaluation_can_run_without_self_healing`,
`test_sw_4_5_failed_and_broken_are_evaluated_but_skipped_targets_are_not`,
`test_sw_4_5_evaluation_agent_cannot_edit`, `test_sw_9_4_evaluation_prompt_keeps_logs_as_data`,
`test_sw_4_5_agent_verdict_is_parsed_from_last_lines`; prova "LLM değerlendirmesi" adımları.

## Python karşılığı
`subprocess.run([...], timeout=…)`; sonuçtaki son `ULAK_TRIAGE:` satırı `json.loads`; `git status --porcelain` denetimi aynı kalmalı.
