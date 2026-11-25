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
1. Koşum: `fix::current_run`. Adaylar: `GET fix-candidates`; hedefi `atla` (`@sample-fail`; flaky'nin hedefi `flaky`'dir ve değerlendirilir) ve
   `acik-mr` olanlar atlanır (`evaluable`). Üst sınır yok; `ULAK_DEGERLENDIR_MAX` verilirse o kadar (`ctx::sinir_ayari`), fazlası kural
   kararıyla kalır. Çalışma klasörü temiz değilse değerlendirme yapılmaz.
2. `.ulak/degerlendir/` silinip yeniden açılır: eski ajan logları yeni kararı etkilemesin.
3. Her senaryo için ekran görüntüsü indirilir, istem (`evaluation_prompt`) kurulur: kurallar + `fix::push_failure`
   (test çıktısı VERİ bloklarında, SW-9.4). `.ulak/degerlendir/istem.md`.
4. Ajan komutu (`evaluation_command`): `ULAK_DEGERLENDIR_CMD`, yoksa `ULAK_FIX_AGENT`'a göre:
   - `opencode`: `<ULAK_OPENCODE_BIN> run --agent <ULAK_DEGERLENDIR_AGENT, degerlendir> [--model <ULAK_OPENCODE_MODEL>] {istem} --file {ekran}`
   - `claude`: `<ULAK_CLAUDE_BIN, claude> -p {istem} --allowedTools Read,Glob,Grep,Skill --disallowedTools Edit,Write,Bash,WebFetch,WebSearch --max-turns 15 --output-format text`
   Zaman aşımı `ULAK_DEGERLENDIR_TIMEOUT` (300 sn). Çıktı `.ulak/degerlendir/<senaryo>-ajan.log`.
5. **Koruma:** ajandan sonra `fix::changed_files`; değişiklik varsa `fix::restore` ile geri alınır,
   karar kullanılmaz.
6. `ULAK_TRIAGE` satırı okunur. Karar yoksa uyarı + bir deneme daha (reddedilen araç oturumu kapatabilir).
   Zaman aşımında o senaryo yeniden denenmez, **atlanır** (kural kararı kalır) ve sıradakine geçilir.
   Log satırı (`fix::zaman_asimi_mesaji`) ajanın o ana kadarki çıktı satır sayısını da yazar: çıktı varsa
   ajan çalışıyordu, süre yetmedi; hiç yoksa model yanıt vermiyor olabilir (Claude Code metin çıktısını
   yalnızca sonunda yazar, onda çıktı olmaması tek başına kanıt değildir). **Art arda**
   `ULAK_ART_ARDA_ZAMAN_ASIMI` (3) zaman aşımında kalanlar denenmez (kota dolunca OpenCode hata vermeden
   bekler; her senaryo yine aynı süreyi bekletirdi). Ajanın başlattığı alt süreçler de sonlandırılır
   (`fix.md`, süreç ağacı).
   İşin başında `ayarlar:` satırı kullanılan sınırları ve kaynaklarını yazar (`ctx::sayi_ayari`:
   "CI değişkeni" / "varsayılan" / sayı değilse uyarı + "varsayılan; değişken geçersiz").
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
