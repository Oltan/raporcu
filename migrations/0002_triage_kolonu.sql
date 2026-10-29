-- SW-4.7: triage kategorisine göre arama için senaryodaki geçerli karar (triage_decisions'tan türetilir)
ALTER TABLE scenarios ADD COLUMN triage_category TEXT;
ALTER TABLE scenarios ADD COLUMN triage_source TEXT;
CREATE INDEX scenarios_triage ON scenarios(triage_category);
-- SW-10.8: disk temizliğinde ek boyutu
ALTER TABLE attachments ADD COLUMN size INTEGER;
