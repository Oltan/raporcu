//! SW-5.3: gece pipeline'ı bitince özet e-posta. Pipeline başına tek e-posta (`emails` tablosu).
//! DRY_RUN'da gönderim simüle edilir: e-posta DATA_DIR/outbox altına yazılır.
//! SMTP: şirket içi relay, düz SMTP (kimlik doğrulama/TLS yok).

use crate::db::now;
use crate::AppState;
use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct Mail {
    pub subject: String,
    pub body: String,
}

pub fn summary(c: &Connection, public_url: &str, project_id: i64, pipeline_id: &str) -> Result<Option<Mail>> {
    let mut st = c.prepare(
        "SELECT r.id, p.path, r.branch, r.total, r.passed, r.failed + r.broken FROM runs r JOIN projects p ON p.id = r.project_id
         WHERE r.project_id = ?1 AND r.pipeline_id = ?2 ORDER BY r.id",
    )?;
    let runs: Vec<(i64, String, String, i64, i64, i64)> = st
        .query_map(params![project_id, pipeline_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    if runs.is_empty() {
        return Ok(None);
    }
    let (passed, failed): (i64, i64) = runs.iter().fold((0, 0), |a, r| (a.0 + r.4, a.1 + r.5));
    let mut body = format!(
        "Gece test koşumu bitti — {} ({})\n\nGeçen: {passed}\nKalan: {failed}\n\n",
        runs[0].1, runs[0].2
    );
    for r in &runs {
        body.push_str(&format!(
            "Rapor #{}: {public_url}/runs/{} — {} senaryo, {} geçti, {} kaldı\n",
            r.0, r.0, r.3, r.4, r.5
        ));
    }
    let mut ms = c.prepare(
        "SELECT f.mr_url, f.mr_iid, f.outcome FROM fix_attempts f JOIN runs r ON r.id = f.run_id
         WHERE r.project_id = ?1 AND r.pipeline_id = ?2 AND f.mr_iid IS NOT NULL ORDER BY f.id",
    )?;
    let mrs: Vec<(Option<String>, i64, String)> = ms
        .query_map(params![project_id, pipeline_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    body.push_str("\nAçılan MR'lar (OpenCode):\n");
    if mrs.is_empty() {
        body.push_str("  yok\n");
    }
    for (u, i, o) in mrs {
        body.push_str(&format!(
            "  !{i} {} {}\n",
            if o == "draft_mr" { "(Draft)" } else { "" },
            u.unwrap_or_default()
        ));
    }
    Ok(Some(Mail {
        subject: format!("[stajproje] {} {}: {passed} geçti, {failed} kaldı", runs[0].1, runs[0].2),
        body,
    }))
}

fn smtp_send(host: &str, port: u16, from: &str, to: &[String], mail: &Mail) -> Result<()> {
    let s = TcpStream::connect((host, port))?;
    s.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut r = BufReader::new(s.try_clone()?);
    let mut w = s;
    let expect = |r: &mut BufReader<TcpStream>, code: &str| -> Result<()> {
        loop {
            let mut line = String::new();
            r.read_line(&mut line)?;
            if line.len() < 4 {
                bail!("SMTP beklenmeyen yanıt: {line}");
            }
            if !line.starts_with(code) {
                bail!("SMTP {code} bekleniyordu: {}", line.trim());
            }
            if line.as_bytes()[3] == b' ' {
                return Ok(());
            }
        }
    };
    expect(&mut r, "220")?;
    write!(w, "HELO stajproje\r\n")?;
    expect(&mut r, "250")?;
    write!(w, "MAIL FROM:<{from}>\r\n")?;
    expect(&mut r, "250")?;
    for t in to {
        write!(w, "RCPT TO:<{t}>\r\n")?;
        expect(&mut r, "25")?;
    }
    write!(w, "DATA\r\n")?;
    expect(&mut r, "354")?;
    let subject = format!("=?UTF-8?B?{}?=", b64(mail.subject.as_bytes()));
    write!(
        w,
        "From: {from}\r\nTo: {}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n",
        to.join(", ")
    )?;
    for line in mail.body.lines() {
        let l = if line.starts_with('.') {
            format!(".{line}")
        } else {
            line.to_string()
        };
        write!(w, "{l}\r\n")?;
    }
    write!(w, ".\r\n")?;
    expect(&mut r, "250")?;
    write!(w, "QUIT\r\n")?;
    Ok(())
}

fn b64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= ch.len() {
                T[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}

/// Pipeline bitince çağrılır. Gönderildiyse true.
pub fn send_for_pipeline(st: &AppState, c: &Connection, project_id: i64, pipeline_id: &str) -> Result<bool> {
    let key = format!("{project_id}:{pipeline_id}");
    let sent: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM emails WHERE pipeline_key = ?1)", [&key], |r| r.get(0))?;
    if sent {
        return Ok(false);
    }
    let Some(mail) = summary(c, &st.cfg.public_url, project_id, pipeline_id)? else {
        return Ok(false);
    };
    let to = st.cfg.mail_to.clone();
    if to.is_empty() {
        tracing::warn!(pipeline_id, "MAIL_TO boş; özet e-posta gönderilmedi");
        return Ok(false);
    }
    let dry = st.cfg.dry_run || st.cfg.smtp_host.is_none();
    if dry {
        let dir = st.paths.outbox_dir();
        std::fs::create_dir_all(&dir)?;
        let file = dir.join(format!(
            "{}-{}.eml",
            key.replace(':', "-"),
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));
        std::fs::write(&file, format!("To: {}\nSubject: {}\n\n{}", to.join(", "), mail.subject, mail.body))?;
        tracing::info!(pipeline_id, alici = to.len(), dosya = %file.display(), "özet e-posta simüle edildi (DRY_RUN)");
    } else {
        smtp_send(
            st.cfg.smtp_host.as_deref().unwrap(),
            st.cfg.smtp_port,
            &st.cfg.mail_from,
            &to,
            &mail,
        )
        .map_err(|e| {
            tracing::warn!(servis = "smtp", hata = %e, deneme = 1, "özet e-posta gönderilemedi");
            e
        })?;
        tracing::info!(pipeline_id, alici = to.len(), "özet e-posta gönderildi");
    }
    c.execute(
        "INSERT INTO emails (pipeline_key, recipients, subject, dry_run, sent_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![key, to.join(","), mail.subject, dry as i64, now()],
    )?;
    Ok(true)
}
