//! `stemwise-cli`: the product without an interface (ENG-7) — schemas, projects, import, render, analysis, AI
//! mix with the fake decider, mixdown and master on a project package.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{self, ImportChoice};
use stemwise_core::{package, wav};

#[derive(Parser)]
#[command(name = "stemwise-cli", version, about = "Stemwise without the interface")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write the JSON Schemas and the TypeScript bindings of the contracts.
    ExportSchemas {
        #[arg(long)]
        schemas: PathBuf,
        #[arg(long)]
        bindings: PathBuf,
    },
    /// Create an empty project package `<parent>/<name>.stemwise`.
    New { parent: PathBuf, name: String },
    /// Import audio files as tracks (roles guessed from the names).
    Import {
        project: PathBuf,
        files: Vec<PathBuf>,
        /// Convert files whose rate differs from the project's (otherwise they are skipped).
        #[arg(long)]
        convert: bool,
    },
    /// Create a demo project with synthetic stems (no audio files are kept outside the package).
    Demo {
        parent: PathBuf,
        #[arg(long, default_value = "Demo")]
        name: String,
        #[arg(long, default_value_t = 20.0)]
        seconds: f64,
    },
    /// Print a project's tracks and routing.
    Info { project: PathBuf },
    /// Analyse the project and print the document sent to the model (ANA-13).
    Analyze { project: PathBuf },
    /// Auto mix with the fake decider (offline demo): print the proposal; apply it with --apply (AI-1, AI-9).
    Automix {
        project: PathBuf,
        #[arg(long, default_value_t = 3)]
        iterations: u32,
        #[arg(long)]
        apply: bool,
        /// Creative ideas to apply with the plan, by id (`i1,i3`) or `all` (AI-27); none by default.
        #[arg(long, value_delimiter = ',')]
        ideas: Vec<String>,
    },
    /// Revision by request with the fake decider (AI-3).
    Revise {
        project: PathBuf,
        request: String,
        #[arg(long)]
        apply: bool,
    },
    /// Master AI with the fake decider on the current mix (MAS-6).
    MasterAi {
        project: PathBuf,
        #[arg(long)]
        request: Option<String>,
        #[arg(long)]
        apply: bool,
    },
    /// AI usage of the project (AI-14): summary, or the call history as CSV.
    AiUsage {
        project: PathBuf,
        #[arg(long)]
        csv: bool,
    },
    /// Render the mix offline to a 32-bit float WAV and print its hash (ENG-4).
    Render {
        project: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 2.0)]
        tail: f64,
    },
}

fn run_ai(project: &Path, iterations: u32, request: Option<String>, apply: bool, ideas: &[String]) -> Result<()> {
    let mut d = load(project)?;
    let opts = stemwise_ai::mixing::MixOptions { max_iterations: iterations, request: request.clone(), scope: None };
    let res = stemwise_ai::mixing::run_mix(
        project,
        &d.project,
        &stemwise_ai::decider::FakeDecider,
        &|_, _| 0.0,
        &opts,
        &|p| eprintln!("{:?} {}/{}", p.phase, p.iteration, p.of),
        &std::sync::atomic::AtomicBool::new(false),
    );
    match res {
        Ok(p) => {
            print_proposal(&p.summary, &p.diff);
            for e in &p.new_elements {
                let what = match e.kind {
                    stemwise_ai::structure::NewKind::Return => "return",
                    stemwise_ai::structure::NewKind::Group => "group",
                };
                println!("  new {what}: {} — {}", e.track.name, e.rationale);
            }
            // AI-27: the ideas, off unless named with --ideas.
            let all: Vec<String> = p.plan.ideas.iter().flat_map(|m| m.keys().cloned()).collect();
            let on: std::collections::HashSet<String> = if ideas.iter().any(|i| i == "all") {
                all.iter().cloned().collect()
            } else {
                ideas.iter().cloned().collect()
            };
            for v in stemwise_ai::dto::idea_views(&p, &d.project, &on) {
                println!("  idea {} {}: {} — {}", v.id, if v.on { "[x]" } else { "[ ]" }, v.name, v.rationale);
                for e in &v.elements {
                    println!("    · {e}");
                }
            }
            for it in &p.iterations {
                println!(
                    "iteration {}: distance {:?} dB{}{}",
                    it.iteration,
                    it.distance_db,
                    if it.valid { "" } else { " (constraints broken)" },
                    if it.best { " best" } else { "" }
                );
            }
            let label = if request.is_some() { "Apply revision" } else { "Apply auto mix" };
            if apply {
                let cmds = if on.is_empty() {
                    p.commands.clone()
                } else {
                    stemwise_ai::mixing::apply_commands(&d.project, &p, None, &on)
                };
                apply_call(project, &mut d, Phase::Mix, label, cmds, p.record)?;
                println!("applied{}", if on.is_empty() { String::new() } else { format!(" with {} ideas", on.len()) });
            } else {
                log_call(
                    &mut d,
                    stemwise_core::model::AiCallRecord {
                        outcome: stemwise_core::model::AiOutcome::Discarded,
                        ..p.record
                    },
                );
                save(project, &d)?;
            }
        }
        Err(e) => {
            log_call(&mut d, e.record().clone());
            save(project, &d)?;
            bail!("{e}");
        }
    }
    Ok(())
}

fn load(dir: &Path) -> Result<Document> {
    Ok(Document::new(package::load(dir).with_context(|| format!("opening {}", dir.display()))?))
}

fn save(dir: &Path, d: &Document) -> Result<()> {
    package::save(dir, &d.project)?;
    Ok(())
}

fn import_files(dir: &Path, d: &mut Document, files: &[PathBuf], convert: bool) -> Result<usize> {
    let cands = import::candidates(&d.project, files);
    let mut choices = Vec::new();
    for c in cands {
        if let Some(e) = c.error {
            eprintln!("skipped {}: {e}", c.path);
            continue;
        }
        choices.push(ImportChoice { path: c.path, role: c.role, name: c.name, convert });
    }
    let n = choices.len();
    let cmds = import::import(dir, &mut d.project, &choices)?;
    d.transact(Phase::Mix, "Import", cmds, None).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(n)
}

/// Synthetic stems for the demo: kick, bass, pad, voice-like tone, all from closed formulas.
fn demo_stems(dir: &Path, seconds: f64) -> Result<Vec<PathBuf>> {
    let rate = 48000u32;
    let n = (seconds * f64::from(rate)) as usize;
    let tau = std::f64::consts::TAU;
    let beat = 60.0 / 96.0;
    let kick: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f64 / f64::from(rate);
            let p = t % beat;
            (0.8 * (-p * 18.0).exp() * (tau * (50.0 + 80.0 * (-p * 30.0).exp()) * p).sin()) as f32
        })
        .collect();
    let bass: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f64 / f64::from(rate);
            let f = [55.0, 55.0, 73.4, 65.4][((t / (beat * 4.0)) as usize) % 4];
            (0.35 * (tau * f * t).sin() + 0.1 * (tau * 2.0 * f * t).sin()) as f32
        })
        .collect();
    let noise = stemwise_dsp::testsignal::xorshift32(11, n * 2);
    let pad: Vec<f32> = (0..n)
        .flat_map(|i| {
            let t = i as f64 / f64::from(rate);
            let s = 0.12 * ((tau * 220.0 * t).sin() + (tau * 277.2 * t).sin() + (tau * 329.6 * t).sin());
            [(s + 0.01 * noise[2 * i]) as f32, (s * 0.9 + 0.01 * noise[2 * i + 1]) as f32]
        })
        .collect();
    let vox: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f64 / f64::from(rate);
            let vib = 1.0 + 0.01 * (tau * 5.0 * t).sin();
            let gate = if (t / beat) % 2.0 < 1.6 { 1.0 } else { 0.0 };
            (0.25 * gate * ((tau * 440.0 * vib * t).sin() + 0.4 * (tau * 880.0 * vib * t).sin())) as f32
        })
        .collect();
    // A snare on the backbeat, and one guitar part played twice: the same strums, another performance.
    let hits = stemwise_dsp::testsignal::xorshift32(12, n);
    let snare: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f64 / f64::from(rate);
            let p = (t + beat) % (beat * 2.0);
            (0.5 * (-p * 22.0).exp() * (0.7 * hits[i] + 0.3 * (tau * 190.0 * p).sin())) as f32
        })
        .collect();
    let guitar = |seed: u32, detune: f64| -> Vec<f32> {
        // Every strum of a performance has its own strength, timing and brightness.
        let pick = stemwise_dsp::testsignal::xorshift32(seed, n);
        let strum = (f64::from(rate) * beat / 2.0) as usize;
        let human = stemwise_dsp::testsignal::xorshift32(seed + 100, 3 * (n / strum + 2));
        (0..n)
            .map(|i| {
                let k = i / strum;
                let late = (0.012 * (1.0 + human[3 * k]) * f64::from(rate)) as usize;
                let Some(since) = (i % strum).checked_sub(late) else { return 0.0 };
                let (t, p) = (i as f64 / f64::from(rate), since as f64 / f64::from(rate));
                let (strength, bright) = (0.7 + 0.3 * human[3 * k + 1], 0.5 + 0.4 * human[3 * k + 2]);
                let chord: f64 = [196.0, 246.9, 293.7, 392.0]
                    .iter()
                    .enumerate()
                    .map(|(h, f)| (1.0 - bright * 0.2 * h as f64) * (tau * f * detune * t).sin())
                    .sum();
                (0.1 * strength * (-p * 7.0).exp() * (chord + (0.3 + bright) * pick[i])) as f32
            })
            .collect()
    };
    let mut files = Vec::new();
    for (name, ch, data) in [
        ("Kick", 1u16, kick),
        ("Snare", 1, snare),
        ("Bass DI", 1, bass),
        ("Gtr Left", 1, guitar(21, 1.0)),
        ("Gtr Right", 1, guitar(22, 1.004)),
        ("Pad", 2, pad),
        ("Lead Vox", 1, vox),
    ] {
        let p = dir.join(format!("{name}.wav"));
        wav::write_f32(&p, ch, rate, &data)?;
        files.push(p);
    }
    Ok(files)
}

/// Applies a proposal as one history step and logs the call (outcome applied).
fn apply_call(
    dir: &Path,
    d: &mut Document,
    phase: Phase,
    label: &str,
    cmds: Vec<stemwise_core::command::Command>,
    mut record: stemwise_core::model::AiCallRecord,
) -> Result<()> {
    d.transact(phase, label, cmds, Some(record.id.clone())).map_err(|e| anyhow::anyhow!("{e}"))?;
    record.outcome = stemwise_core::model::AiOutcome::Applied;
    record.history_label = Some(label.to_string());
    log_call(d, record);
    save(dir, d)
}

/// The app's data folder, shared with the desktop app (cost register, AI-14): `STEMWISE_DATA` overrides it.
fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("STEMWISE_DATA") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    home.join("Library/Application Support/app.stemwise.desktop")
}

fn log_call(d: &mut Document, record: stemwise_core::model::AiCallRecord) {
    // Every call, applied, discarded, failed or cancelled, enters the register of all projects (AI-14, AI-15).
    stemwise_ai::log::register_cost(
        &data_dir(),
        &stemwise_ai::log::CostEntry {
            project: d.project.name.clone(),
            call: record.id.clone(),
            at_ms: record.started_ms,
            model: record.model.clone(),
            cost_usd: record.cost_usd,
            estimated: record.cost_estimated,
        },
    );
    let mut calls = d.project.ai.calls.clone();
    calls.retain(|c| c.id != record.id);
    calls.push(record);
    let _ = d.apply_view(vec![stemwise_core::command::Command::SetAiCalls { calls }]);
}

fn print_proposal(summary: &str, diff: &[stemwise_ai::apply::TrackDiff]) {
    println!("summary: {summary}");
    for t in diff {
        println!("  {}{}", t.name, t.note.map(|n| format!(" [{n}]")).unwrap_or_default());
        for r in &t.rows {
            let f = |v: Option<f64>| v.map_or("off".to_string(), |x| format!("{x:.1}"));
            match &r.text {
                // A block the plan adds (AI-25): what it is set to, in words.
                Some(t) => println!("    {:<24} — → {t}", r.label),
                None => println!("    {:<24} {} → {} {}", r.label, f(r.before), f(r.after), r.unit),
            }
        }
        if !t.rationale.is_empty() {
            println!("    why: {}", t.rationale);
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::ExportSchemas { schemas, bindings } => {
            stemwise_core::schema::write_schemas(&schemas)?;
            for (name, text) in stemwise_ai::plan::json_schemas() {
                std::fs::write(schemas.join(name), text)?;
            }
            stemwise_core::schema::write_bindings(&bindings)?;
            stemwise_engine::dto::write_bindings(&bindings)?;
            stemwise_ai::dto::write_bindings(&bindings)?;
            stemwise_export::dto::write_bindings(&bindings)?;
            std::fs::write(
                bindings.join("param_specs.json"),
                serde_json::to_string_pretty(&stemwise_core::params::all_specs())? + "\n",
            )?;
            std::fs::write(
                bindings.join("fx_catalog.json"),
                serde_json::to_string_pretty(&stemwise_engine::chain::catalog())? + "\n",
            )?;
            // UI-15: the families of the Explain map are the ones the balance is measured on (AI-22).
            let of: std::collections::BTreeMap<&str, &str> = stemwise_core::role::Role::ALL
                .iter()
                .map(|r| (r.as_str(), stemwise_ai::balance::family_of(Some(*r))))
                .collect();
            std::fs::write(
                bindings.join("role_families.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "families": stemwise_ai::balance::FAMILIES,
                    "of": of,
                }))? + "\n",
            )?;
            println!("schemas written to {} and bindings to {}", schemas.display(), bindings.display());
        }
        Cmd::New { parent, name } => {
            let (dir, _) = package::create(&parent, &name)?;
            println!("{}", dir.display());
        }
        Cmd::Import { project, files, convert } => {
            let mut d = load(&project)?;
            let n = import_files(&project, &mut d, &files, convert)?;
            save(&project, &d)?;
            println!("imported {n} file(s)");
        }
        Cmd::Demo { parent, name, seconds } => {
            let (dir, p) = package::create(&parent, &name)?;
            let mut d = Document::new(p);
            let tmp = dir.join("cache").join("demo-src");
            std::fs::create_dir_all(&tmp)?;
            let files = demo_stems(&tmp, seconds)?;
            import_files(&dir, &mut d, &files, false)?;
            std::fs::remove_dir_all(&tmp)?;
            // Two sections, so plans can differ between them (AI-6).
            let end = d.project.content_end();
            for (name, start, end) in [("Verse", 0, end / 2), ("Chorus", end / 2, end)] {
                let cmds = stemwise_core::edit::run(
                    &mut d.project,
                    &stemwise_core::edit::Edit::AddSection { name: name.into(), start, end },
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                d.transact(Phase::Mix, "Add section", cmds, None).map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            save(&dir, &d)?;
            println!("{}", dir.display());
        }
        Cmd::Info { project } => {
            let d = load(&project)?;
            let p = &d.project;
            println!("{} · {:?} Hz · {} tracks · {} sections", p.name, p.sample_rate, p.tracks.len(), p.sections.len());
            for t in &p.tracks {
                println!(
                    "  {:<5} {:<8} {:<20} out {:?} · fader {:+.1} dB",
                    t.id,
                    format!("{:?}", t.kind),
                    t.name,
                    t.output,
                    t.chain.fader_db
                );
            }
        }
        Cmd::Analyze { project } => {
            let d = load(&project)?;
            let a = stemwise_ai::analysis_doc::analyze_project(&project, &d.project, &|_| true)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let doc = stemwise_ai::analysis_doc::model_document(
                &a,
                &d.project,
                &stemwise_ai::analysis_doc::Ids::of(&d.project),
            );
            println!("{}", serde_json::to_string_pretty(&doc)?);
            eprintln!("estimated tokens: {}", stemwise_ai::analysis_doc::estimated_tokens(&doc));
        }
        Cmd::Automix { project, iterations, apply, ideas } => run_ai(&project, iterations, None, apply, &ideas)?,
        Cmd::Revise { project, request, apply } => run_ai(&project, 1, Some(request), apply, &[])?,
        Cmd::MasterAi { project, request, apply } => {
            let mut d = load(&project)?;
            let r = stemwise_engine::offline::render_song(&project, &d.project, 1.0)?;
            let x = r.interleaved();
            let tp = stemwise_dsp::truepeak::true_peak_db(&x, 2);
            let g = 10f64.powf(((-3.01) - tp).min(0.0) / 20.0);
            let x: Vec<f32> = x.iter().map(|v| (f64::from(*v) * g) as f32).collect();
            let mixdown = stemwise_ai::mastering::measure_mixdown(&x, r.rate);
            let a = stemwise_ai::analysis_doc::analyze_project(&project, &d.project, &|_| true)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let input = stemwise_ai::mastering::MasterInput {
                samples: &x,
                rate: r.rate,
                measures: &mixdown,
                target: a.target.as_ref(),
                references: Vec::new(),
                start_gain_db: None,
            };
            let res = stemwise_ai::mastering::run_master(
                &project,
                &d.project,
                &input,
                &stemwise_ai::decider::FakeDecider,
                &|_, _| 0.0,
                request,
                &|_| {},
                &std::sync::atomic::AtomicBool::new(false),
            );
            match res {
                Ok(p) => {
                    print_proposal(&p.summary, std::slice::from_ref(&p.diff));
                    if apply {
                        apply_call(&project, &mut d, Phase::Master, "Apply master proposal", p.commands, p.record)?;
                        println!("applied");
                    } else {
                        log_call(
                            &mut d,
                            stemwise_core::model::AiCallRecord {
                                outcome: stemwise_core::model::AiOutcome::Discarded,
                                ..p.record
                            },
                        );
                        save(&project, &d)?;
                    }
                }
                Err(e) => {
                    log_call(&mut d, e.record().clone());
                    save(&project, &d)?;
                    bail!("{e}");
                }
            }
        }
        Cmd::AiUsage { project, csv } => {
            let d = load(&project)?;
            let calls = &d.project.ai.calls;
            if csv {
                print!("{}", stemwise_ai::log::calls_csv(calls));
            } else {
                let cost: f64 = calls.iter().map(|c| c.cost_usd).sum();
                let (ti, to): (u64, u64) = calls.iter().fold((0, 0), |(a, b), c| (a + c.tokens_in, b + c.tokens_out));
                println!("{} calls · {ti} tokens in · {to} tokens out · ${cost:.4}", calls.len());
                for c in calls {
                    println!(
                        "  {} {:?} {} attempts {} ${:.4}{} {:?}",
                        c.id,
                        c.kind,
                        c.model,
                        c.attempts,
                        c.cost_usd,
                        if c.cost_estimated { " (est.)" } else { "" },
                        c.outcome
                    );
                }
            }
        }
        Cmd::Render { project, out, tail } => {
            let d = load(&project)?;
            if d.project.sample_rate.is_none() {
                bail!("nothing to render: the project has no audio");
            }
            let r = stemwise_engine::offline::render_song(&project, &d.project, tail)?;
            if let Some(out) = out {
                wav::write_f32(&out, 2, r.rate, &r.interleaved())?;
            }
            println!("frames {} · hash {}", r.left.len(), r.hash());
        }
    }
    Ok(())
}
