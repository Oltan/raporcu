# divan/src/hooks.rs

**Amaç:** Olay sonrası işler.

- `after_results(st, koşum)`: canlı ekrana "değişti" sinyali (WebSocket yayını).
- `after_finish(st, koşum)`: bellekteki canlı "şu an" durumunu siler; WebSocket sinyali; triage
  kurallarını uygular (`triage::apply_rules`); Jira eşleşmesini **arka planda** başlatır
  (`jira::process_run`, ağ çağrıları içerir; Jira ayarlı değilse yalnızca tag bağlantıları).

Çağıranlar: zip yükleme (yeni koşum), `/finish`, zaman aşımı döngüsü.
