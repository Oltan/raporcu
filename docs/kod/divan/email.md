# divan/src/email.rs

**Amaç:** SW-5.3 — gece pipeline'ı bitince pipeline başına **tek** özet e-posta.

- Tetik: webhook'ta `object_kind=pipeline`, durum `success|failed`, kaynak `schedule`.
- `summary(c, public_url, proje, pipeline)`: o pipeline'ın koşumları; konu
  `[divan] <proje> <branch>: N geçti, M kaldı`; gövde: toplamlar, her koşumun rapor linki,
  açılan OpenCode MR'ları (Draft işaretli) ya da "yok".
- `send_for_pipeline(st, c, proje, pipeline)`: `emails.pipeline_key = "<proje>:<pipeline>"` varsa
  gönderilmez. `MAIL_TO` boşsa WARN. `DRY_RUN` ya da `SMTP_HOST` yoksa `outbox/<anahtar>-<zaman>.eml`
  dosyasına yazılır ("simüle edildi"). Değilse düz SMTP: `HELO`, `MAIL FROM`, her alıcı `RCPT TO`,
  `DATA` (konu UTF-8 base64 kodlu, gövde 8bit, `.` ile başlayan satır `..`), `QUIT`. Kayıt eklenir.

## Testler
`test_sw_5_3_nightly_summary_email_once_per_pipeline_dry_run`, `test_sw_5_3_smtp_send_when_not_dry_run`.

## Python karşılığı
`smtplib.SMTP` + `email.message.EmailMessage`.
