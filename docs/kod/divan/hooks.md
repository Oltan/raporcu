# divan/src/hooks.rs

**Amaç:** Olay sonrası işler.

- `after_results(st, koşum)`: canlı ekrana "değişti" sinyali (WebSocket yayını).
- `after_finish(st, koşum)`: bellekteki canlı "şu an" durumunu siler; WebSocket sinyali; triage
  kurallarını uygular (`triage::apply_rules`); Jira eşleşmesini **arka planda** başlatır
  (`jira::process_run`, ağ çağrıları içerir; Jira ayarlı değilse yalnızca tag bağlantıları), ardından
  `refresh_jira_issues`: koşumdaki Jira anahtarlarının (tag + bağlı bug) bilgisi Jira'dan çekilir
  (`jira::issue_keys_to_refresh` 30 dk / 200; ağ çağrıları ayrı iş parçacığında, DB kilidi tutulmadan;
  token reddi ya da bağlantı hatasında kalanlar bırakılır), `jira_issues`'a yazılır.

Çağıranlar: zip yükleme (yeni koşum), `/finish`, zaman aşımı döngüsü.
