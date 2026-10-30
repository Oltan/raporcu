# Self-healing skill'leri

Test projesinde **`.claude/skills/`** klasörüne kopyalanır:

```
cp -r ci/skills/testfx-secici ci/skills/cucumber-adim ci/skills/hata-siniflandirma <test-projesi>/.claude/skills/
```

Bu klasörü hem **OpenCode** (proje skill yolları: `.opencode/skills/`, `.claude/skills/`, `.agents/skills/`)
hem **Claude Code** (`.claude/skills/`) okur; iki ajan için tek kopya yeter.

| Skill | Ne zaman yüklenir |
|-------|-------------------|
| `hata-siniflandirma` | her zaman, değişiklikten önce: test kodu / ürün / ortam / flaky kararı, `ULAK_TRIAGE` |
| `testfx-secici` | seçici, bekleme, düğüm bulunamadı hataları |
| `cucumber-adim` | undefined/ambiguous adım, adım tanımı ve feature kuralları |

Kendi projenize özel bilgi (sayfa nesneleri, ortak yardımcı sınıflar, ekran akışları) için aynı
biçimde yeni bir skill ekleyin: `.claude/skills/<ad>/SKILL.md`, başında `name` (klasör adıyla aynı,
küçük harf ve tire) ve `description` (ne zaman yükleneceği). Ajanlar skill listesini açıklamalarıyla
görür ve gerektiğinde `skill` aracıyla yükler; `fix` ajanının talimatı da adlarıyla yüklemesini söyler.

Dosya biçimi (OpenCode ve Claude Code aynı):

```markdown
---
name: sayfa-nesneleri
description: Projedeki sayfa nesnesi sınıfları ve ortak yardımcılar. Yeni seçici ya da adım yazarken yükle.
---

# İçerik: kurallar, örnek kod, yapılmayacaklar
```
