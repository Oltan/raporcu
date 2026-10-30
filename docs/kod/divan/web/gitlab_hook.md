# divan/src/web/gitlab_hook.rs

**Amaç:** `POST /api/gitlab/webhook` (SW-3.6, 9.3, 5.3, 7.5).

1. `X-Gitlab-Token` başlığı `GITLAB_WEBHOOK_TOKEN` ile sabit zamanlı karşılaştırılır; ayarlı değil
   ya da yanlışsa **401** + WARN.
2. Gövde JSON değilse 400.
3. `object_kind`:
   - `pipeline` → `gitlab::apply_pipeline_event`; durum `success|failed` ve kaynak `schedule` ise
     `email::send_for_pipeline` (hata e-postayı düşürür, webhook'u değil).
   - `merge_request` → `gitlab::apply_mr_event`.
   - diğer → yok sayılır.
4. 200 "ok".

GitLab ayarı: proje → Webhooks, Pipeline events + Merge request events.
