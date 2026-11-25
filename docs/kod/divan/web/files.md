# divan/src/web/files.rs

**Amaç:** Ek dosyalarını sunma (SW-4.1, 10.6).

`GET /files/{id}`: ek kaydı yoksa 404; silinmişse **410** + neden ("Silindi, bkz. STAJ-42");
dosya henüz gelmediyse 404. Var ise kayıttaki MIME türüyle, **HTTP Range** destekli (video parça
parça; `Range: bytes=0-99` → 206), `Cache-Control: private, max-age=86400`.
`serve_attachment_file` paylaşım yolu (`/s/{token}/files/{id}`) tarafından da kullanılır.
