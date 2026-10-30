//! SW-2.4: tüm çalışma verisinin yolları. DATA_DIR dışında hiçbir yere yazılmaz; başka modüller
//! yol kurmaz, buradan alır.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub fn new(data_dir: impl Into<PathBuf>) -> Paths {
        Paths { root: data_dir.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db(&self) -> PathBuf {
        self.root.join("raporcu.db")
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.root.join("backup")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    pub fn runs_dir(&self) -> PathBuf {
        self.root.join("runs")
    }

    pub fn run_dir(&self, run_id: i64) -> PathBuf {
        self.runs_dir().join(run_id.to_string())
    }

    /// Allure ekleri (ekran görüntüsü, video, metin) — dosya adı Allure `source` alanıdır.
    pub fn run_files_dir(&self, run_id: i64) -> PathBuf {
        self.run_dir(run_id).join("files")
    }

    pub fn attachment(&self, run_id: i64, source: &str) -> PathBuf {
        self.run_files_dir(run_id).join(source)
    }

    /// DRY_RUN e-postaları (SW-5.3)
    pub fn outbox_dir(&self) -> PathBuf {
        self.root.join("outbox")
    }

    /// Sunulan stajci ikilileri (SW-10.9)
    pub fn tools_dir(&self) -> PathBuf {
        self.root.join("tools")
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for d in [
            self.root.clone(),
            self.backup_dir(),
            self.logs_dir(),
            self.tmp_dir(),
            self.runs_dir(),
            self.outbox_dir(),
            self.tools_dir(),
        ] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_2_4_all_paths_under_data_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let p = Paths::new(tmp.path().join("veri"));
        p.ensure().unwrap();
        for path in [
            p.db(),
            p.backup_dir(),
            p.logs_dir(),
            p.tmp_dir(),
            p.runs_dir(),
            p.run_files_dir(7),
            p.attachment(7, "a-attachment.png"),
            p.outbox_dir(),
            p.tools_dir(),
        ] {
            assert!(path.starts_with(tmp.path().join("veri")), "{path:?}");
        }
        assert!(p.backup_dir().is_dir());
    }
}
