-- Test edilen uygulamanın sürümü (ulak: ULAK_UYGULAMA_SURUMU / UYGULAMA_SURUMU). Eski koşumlarda boş.
ALTER TABLE runs ADD COLUMN app_version TEXT;
