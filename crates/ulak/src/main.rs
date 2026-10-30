//! ulak: divan CI yardımcı aracı. CI şablonu yalnızca bu alt komutları çağırır.

mod client;
mod ctx;
mod fix;
mod gherkin;
mod preflight;
mod runner;
mod upload;
mod watch;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ulak", version, about = "divan CI aracı")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Testlerden önce test projesini kontrol eder (pom, Allure eklentisi, features.txt)
    Preflight,
    /// Koşumu açar, izleyiciyi başlatır ve testleri koşar (Linux: Xvfb)
    Start {
        /// Yalnızca koşumu aç, testleri koşma
        #[arg(long)]
        no_tests: bool,
    },
    /// allure-results klasörünü izleyip yeni sonuçları hemen gönderir
    Watch {
        #[arg(long)]
        run_id: Option<i64>,
    },
    /// Sonuçları zip olarak gönderir; ulaşılamazsa kuyruğa yazar
    Upload {
        /// Kuyruktaki (artifact) yüklemeleri gönder
        #[arg(long)]
        resume: bool,
        /// Kuyruk klasörü (varsayılan .ulak/queue)
        #[arg(long)]
        queue: Option<std::path::PathBuf>,
    },
    /// Koşumu bitirir
    Finish {
        #[arg(long, default_value = "finished")]
        status: String,
    },
    /// Test kodu hatalarını OpenCode ile düzeltir, MR açar
    Fix,
}

fn main() {
    let cli = Cli::parse();
    let c = ctx::Ctx::from_env();
    let code = match cli.cmd {
        Cmd::Preflight => Ok(preflight::run(&c.project_dir)),
        Cmd::Start { no_tests } => runner::start(&c, !no_tests),
        Cmd::Watch { run_id } => match run_id.or_else(|| c.saved_run().map(|r| r.run_id)) {
            Some(id) => Ok(watch::watch(&c, id)),
            None => Err(anyhow::anyhow!("run_id yok (.ulak/run.json bulunamadı)")),
        },
        Cmd::Upload { resume, queue } => {
            if resume {
                upload::resume(&c, queue)
            } else {
                upload::upload(&c)
            }
        }
        Cmd::Finish { status } => upload::finish(&c, &status),
        Cmd::Fix => fix::fix(&c),
    };
    match code {
        Ok(c) => std::process::exit(c),
        Err(e) => {
            ctx::say(format!("HATA: {e:#}"));
            std::process::exit(1)
        }
    }
}
