//! Läget i sekvenseringspipelinen, läst ur CGU-verktygens lager.
//!
//! DUCTUS mailar en statusrad per steg - nästan hälften av inkorgen. De
//! parsas till händelser av `pipeline.py` i ai-testprojekt och hamnar i en
//! SQLite-fil tillsammans med sammanfattningarna av den övriga posten.
//!
//! **Den här modulen läser bara. Den rör aldrig Outlook och aldrig en modell.**
//! Insamlingen sker i en schemalagd körning (launchd, `daily`); appen visar
//! det som redan står i lagret. Det är därför panelen är omedelbar och
//! fungerar utan att någonting annat är igång.
//!
//! Lagret ligger i dag som en fil på samma maskin. Ska flera se samma bild
//! måste det flytta till en tjänst - då byts `open_store` mot ett HTTP-anrop
//! och resten av modulen kan stå kvar.

use chrono::{Duration, Local, NaiveDateTime};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Var CGU-verktygens lager ligger när inget annat anges.
const DEFAULT_STORE: &str = "dev/ai-testprojekt/store.db";

/// Hur länge ett prov får vara tyst efter en start innan det räknas som
/// hängande. Samma tröskel som `pipeline.OPEN_HOURS`.
const OPEN_HOURS: i64 = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvent {
    pub sent: String,
    pub status: String,
    pub sample: String,
    pub wp: String,
    pub assay: String,
    pub folder: String,
    pub execution: String,
    pub source: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleState {
    pub sample: String,
    pub state: String,
    pub note: String,
    pub wp: String,
    pub assay: String,
    pub first: String,
    pub last: String,
    pub events: usize,
    pub folder: String,
    pub execution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailSummary {
    pub subject: String,
    pub sender: String,
    pub kind: String,
    pub lage: String,
    pub datum: String,
    pub covers_to: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeskStatus {
    pub store: String,
    pub samples: Vec<SampleState>,
    pub counts: HashMap<String, usize>,
    pub mail: Vec<MailSummary>,
    pub timeline: Vec<PipelineEvent>,
}

fn default_store() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(DEFAULT_STORE)
}

fn open_store(path: &PathBuf) -> Result<Connection, String> {
    if !path.exists() {
        return Err(format!(
            "Hittar inget lager på {}. Kör CGU-verktygens ./daily först.",
            path.display()
        ));
    }
    // Läsläge: appen ska aldrig kunna skriva i insamlingens lager, inte ens
    // av misstag. Skrivaren är den schemalagda körningen, ingen annan.
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Kunde inte öppna {}: {e}", path.display()))
}

/// Läget för ett prov, räknat ur dess händelser.
///
/// OBS: regeln finns också i `pipeline.state()` i ai-testprojekt, och det är
/// en kopia - inte en andra sanning. Ändras den där måste den ändras här.
/// Det är priset för att appen läser råa händelser; rätt lösning på sikt är
/// att insamlingen skriver ned läget så att det bara finns på ett ställe.
fn state_for(events: &[PipelineEvent], now: NaiveDateTime) -> (String, String) {
    let mut when: HashMap<&str, String> = HashMap::new();
    for e in events {
        let slot = when.entry(e.status.as_str()).or_default();
        if e.sent > *slot {
            *slot = e.sent.clone();
        }
    }
    let errors = events.iter().filter(|e| e.status == "ERROR").count();
    let last = events.last().map(|e| e.sent.clone()).unwrap_or_default();
    let age_hours = NaiveDateTime::parse_from_str(&last, "%Y-%m-%dT%H:%M:%S")
        .map(|t| (now - t).num_hours())
        .unwrap_or(0);

    if events.iter().all(|e| e.status == "INFO") {
        return ("notis".into(), "sample sheet".into());
    }

    let flera = |n: usize, en: &str, m: &str| if n > 1 { m.to_string() } else { en.to_string() };

    if let Some(fel) = when.get("ERROR") {
        // Ett fel står kvar tills något senare gått igenom. En arkivering av
        // halvfärdiga filer får inte dölja att analysen aldrig blev av.
        let klart = ["SUCCESS", "DONE"]
            .iter()
            .filter_map(|s| when.get(*s))
            .max()
            .cloned()
            .unwrap_or_default();
        if klart > *fel {
            return (
                "klar".into(),
                flera(errors, "klar efter fel", &format!("klar efter {errors} fel")),
            );
        }
        // En ny start efter felet betyder att någon redan kört om - men en
        // omkörning som tystnat är fortfarande en omkörning som tystnat.
        if when.get("STARTED").map(|s| s > fel).unwrap_or(false) {
            let omkörd = flera(errors, "omkörd efter fel", &format!("omkörd efter {errors} fel"));
            if age_hours > OPEN_HOURS {
                return (
                    "hänger".into(),
                    format!("{omkörd}, tyst i {} dygn", age_hours / 24),
                );
            }
            return ("pågår".into(), omkörd);
        }
        return ("fel".into(), flera(errors, "fel", &format!("{errors} fel")));
    }
    if when.contains_key("SUCCESS") {
        return ("klar".into(), "analys klar".into());
    }
    if when.contains_key("DONE") {
        return ("klar".into(), "arkiverad".into());
    }
    if when.contains_key("STARTED") {
        if age_hours > OPEN_HOURS {
            return ("hänger".into(), format!("startad för {age_hours} h sedan"));
        }
        return ("pågår".into(), "startad".into());
    }
    if when.contains_key("WAITING") {
        return ("köad".into(), "väntar på tur".into());
    }
    ("pågår".into(), "startad".into())
}

fn rank(state: &str) -> usize {
    match state {
        "fel" => 0,
        "hänger" => 1,
        "pågår" => 2,
        "köad" => 3,
        "klar" => 4,
        _ => 5,
    }
}

fn read_events(con: &Connection, since: &str) -> Result<Vec<PipelineEvent>, String> {
    let mut stmt = con
        .prepare(
            "SELECT sent, status, COALESCE(sample, ''), COALESCE(run, ''), \
             COALESCE(wp, ''), COALESCE(assay, ''), COALESCE(folder, ''), \
             COALESCE(execution, ''), COALESCE(source, ''), COALESCE(subject, '') \
             FROM pipeline_events WHERE sent >= ?1 ORDER BY sent",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([since], |r| {
            let sample: String = r.get(2)?;
            let run: String = r.get(3)?;
            Ok(PipelineEvent {
                sent: r.get(0)?,
                status: r.get(1)?,
                sample: if sample.is_empty() { run } else { sample },
                wp: r.get(4)?,
                assay: r.get(5)?,
                folder: r.get(6)?,
                execution: r.get(7)?,
                source: r.get(8)?,
                subject: r.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn read_mail(con: &Connection, since: &str) -> Result<Vec<MailSummary>, String> {
    // Senaste sammanfattningen per tråd, samma urval som postvyn gör.
    let mut stmt = con
        .prepare(
            "SELECT s.subject, s.sender, s.kind, s.lage, s.datum, s.covers_to, s.model \
             FROM mail_summaries s JOIN (SELECT thread_key, MAX(generated_at) AS g \
             FROM mail_summaries GROUP BY thread_key) m \
             ON s.thread_key = m.thread_key AND s.generated_at = m.g \
             WHERE s.covers_to >= ?1 ORDER BY s.score DESC, s.covers_to DESC LIMIT 50",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([since], |r| {
            Ok(MailSummary {
                subject: r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                sender: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                kind: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                lage: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                datum: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                covers_to: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
                model: r.get::<_, Option<String>>(6)?.unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Läget i pipelinen plus den sammanfattade posten för perioden.
#[tauri::command]
pub fn cgu_desk_status(days: Option<i64>, store: Option<String>) -> Result<DeskStatus, String> {
    let days = days.unwrap_or(7).clamp(1, 365);
    let path = store.map(PathBuf::from).unwrap_or_else(default_store);
    let con = open_store(&path)?;

    let now = Local::now().naive_local();
    let since = (now - Duration::days(days))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let events = read_events(&con, &since)?;
    let mut grouped: HashMap<String, Vec<PipelineEvent>> = HashMap::new();
    for e in &events {
        if e.sample.is_empty() {
            continue;
        }
        grouped.entry(e.sample.clone()).or_default().push(e.clone());
    }

    let mut samples: Vec<SampleState> = grouped
        .into_iter()
        .map(|(sample, mut hist)| {
            hist.sort_by(|a, b| a.sent.cmp(&b.sent));
            let (state, note) = state_for(&hist, now);
            // Arkiveringsnotiserna bär ingen sökväg. Ta den senaste som har en.
            let pick = |f: fn(&PipelineEvent) -> &String| {
                hist.iter().rev().map(f).find(|v| !v.is_empty()).cloned().unwrap_or_default()
            };
            SampleState {
                wp: hist.iter().map(|e| &e.wp).find(|v| !v.is_empty()).cloned().unwrap_or_default(),
                assay: hist.iter().map(|e| &e.assay).find(|v| !v.is_empty()).cloned().unwrap_or_default(),
                first: hist.first().map(|e| e.sent.clone()).unwrap_or_default(),
                last: hist.last().map(|e| e.sent.clone()).unwrap_or_default(),
                events: hist.len(),
                folder: pick(|e| &e.folder),
                execution: pick(|e| &e.execution),
                sample,
                state,
                note,
            }
        })
        .collect();
    samples.sort_by(|a, b| rank(&a.state).cmp(&rank(&b.state)).then(b.last.cmp(&a.last)));

    let mut counts: HashMap<String, usize> = HashMap::new();
    for s in &samples {
        *counts.entry(s.state.clone()).or_insert(0) += 1;
    }

    Ok(DeskStatus {
        store: path.display().to_string(),
        mail: read_mail(&con, &since)?,
        timeline: events,
        samples,
        counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(sent: &str, status: &str) -> PipelineEvent {
        PipelineEvent {
            sent: sent.into(),
            status: status.into(),
            sample: "X".into(),
            wp: "wp2".into(),
            assay: "tm".into(),
            folder: String::new(),
            execution: String::new(),
            source: "mail".into(),
            subject: String::new(),
        }
    }

    fn now() -> NaiveDateTime {
        NaiveDateTime::parse_from_str("2026-09-17T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
    }

    // Fallen nedan speglar `pipeline.state()` i ai-testprojekt. Går de isär
    // är det kopian här som ska rättas, inte testet.

    #[test]
    fn fel_star_kvar_nar_inget_gatt_igenom_efter() {
        let hist = [ev("2026-09-17T08:00:00", "STARTED"), ev("2026-09-17T09:00:00", "ERROR")];
        assert_eq!(state_for(&hist, now()).0, "fel");
    }

    #[test]
    fn arkivering_efter_felet_gor_provet_klart() {
        let hist = [ev("2026-09-17T09:00:00", "ERROR"), ev("2026-09-17T10:00:00", "DONE")];
        let (state, note) = state_for(&hist, now());
        assert_eq!(state, "klar");
        assert_eq!(note, "klar efter fel");
    }

    #[test]
    fn arkivering_fore_felet_doljer_det_inte() {
        let hist = [ev("2026-09-17T08:00:00", "DONE"), ev("2026-09-17T09:00:00", "ERROR")];
        assert_eq!(state_for(&hist, now()).0, "fel");
    }

    #[test]
    fn fardsk_omkorning_efter_fel_pagar() {
        let hist = [ev("2026-09-17T09:00:00", "ERROR"), ev("2026-09-17T10:00:00", "STARTED")];
        let (state, note) = state_for(&hist, now());
        assert_eq!(state, "pågår");
        assert_eq!(note, "omkörd efter fel");
    }

    #[test]
    fn tystnad_omkorning_hanger() {
        // TE430: två fel, omstart två minuter senare, sedan tyst i tjugo dygn.
        let hist = [
            ev("2026-08-27T15:16:00", "ERROR"),
            ev("2026-08-27T15:17:00", "ERROR"),
            ev("2026-08-27T15:18:00", "STARTED"),
        ];
        let (state, note) = state_for(&hist, now());
        assert_eq!(state, "hänger");
        assert!(note.starts_with("omkörd efter 2 fel, tyst i 20"), "fick: {note}");
    }

    #[test]
    fn bara_info_ar_en_notis() {
        let hist = [ev("2026-09-17T09:00:00", "INFO")];
        assert_eq!(state_for(&hist, now()).0, "notis");
    }

    #[test]
    fn waiting_ar_koad_inte_pagaende() {
        let hist = [ev("2026-09-17T09:00:00", "WAITING")];
        let (state, note) = state_for(&hist, now());
        assert_eq!(state, "köad");
        assert_eq!(note, "väntar på tur");
    }

    #[test]
    fn start_utan_fortsattning_hanger_efter_troskeln() {
        let hist = [ev("2026-09-15T09:00:00", "STARTED")];
        assert_eq!(state_for(&hist, now()).0, "hänger");
    }
}
