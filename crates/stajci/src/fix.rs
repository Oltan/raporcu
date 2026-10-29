//! `stajci fix`: faz 8'de doldurulacak.
use crate::ctx::Ctx;

pub fn fix(_c: &Ctx) -> anyhow::Result<i32> {
    crate::ctx::say("fix: düzeltilecek senaryo yok");
    Ok(0)
}
