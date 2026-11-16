// Self-test of the real app (A-011): runs the main flows through the Tauri backend and the real engine, then
// reports to the shell. Only active when the app is started with STEMWISE_SELFTEST_PROJECT.
import { invoke } from "@tauri-apps/api/core";
import type { AiStatus, Backend, MasterStatus, MeterSnapshot, ProjectEvent } from "./backend/types";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const sleepThen = async <T,>(f: () => Promise<T>) => {
  await sleep(150);
  return f();
};

export async function runSelfTestIfRequested(backend: Backend) {
  if (backend.kind !== "tauri") return;
  const project = await invoke<string | null>("selftest_config");
  if (!project) return;
  const steps: { step: string; ok: boolean; detail: string }[] = [];
  const check = (step: string, ok: boolean, detail = "") => steps.push({ step, ok, detail });
  let last: MeterSnapshot | null = null;
  let meterCount = 0;
  let ev: ProjectEvent | null = null;
  const offM = backend.onMeters((m) => {
    last = m;
    meterCount++;
  });
  const offP = backend.onProject((e) => (ev = e));
  try {
    await backend.openProject(project);
    const p = await backend.getProject();
    check("open", p.tracks.length > 0, `${p.name}: ${p.tracks.length} tracks`);
    const t0 = await backend.transportState();
    check("audio device", !!t0.device && !t0.deviceError, `${t0.device} @ ${t0.deviceRate} Hz ${t0.deviceError ?? ""}`);
    await backend.play();
    meterCount = 0;
    await sleep(2000);
    const t1 = await backend.transportState();
    check("play advances", t1.playing && t1.frame > 0.5 * t1.rate, `frame ${t1.frame}`);
    check("meters ≥ 30 Hz", meterCount >= 55, `${meterCount} frames in 2 s`);
    const bus = (last as MeterSnapshot | null)?.bus[0] ?? -200;
    check("bus has signal", bus > -60, `bus peak ${bus.toFixed(1)} dBFS`);
    const vox = p.tracks.find((t) => t.kind === "audio")!;
    await backend.beginGesture("Move level");
    for (const v of [-6, -12, -24, -48]) {
      await backend.gestureParam(vox.id, "fader", v);
      await sleep(40);
    }
    await backend.endGesture(false);
    await sleep(500);
    const after = (ev as ProjectEvent | null)?.project.tracks.find((t) => t.id === vox.id)?.chain.fader_db;
    check("fader from the UI", after === -48, `fader ${after}`);
    const lvl = (last as MeterSnapshot | null)?.tracks.find((t) => t[0] === vox.id)?.[1][2] ?? 0;
    check("engine follows the fader", lvl < -40, `track rms ${lvl.toFixed(1)} dBFS`);
    await backend.undo("mix");
    await sleep(200);
    const undone = (ev as ProjectEvent | null)?.project.tracks.find((t) => t.id === vox.id)?.chain.fader_db;
    check("undo", undone === vox.chain.fader_db, `fader ${undone}`);
    // F6: edits from the UI reach the core as one step each, and gestures recompute from their start.
    await backend.watchSpectrum(vox.id);
    await sleep(600);
    const spec = (last as MeterSnapshot | null)?.spectrum ?? null;
    check("live spectrum of the selected track", !!spec && spec.length === 96 && Math.max(...spec) > -90, spec ? `max ${Math.max(...spec).toFixed(1)} dB` : "none");
    const clipsBefore = p.tracks.find((t) => t.id === vox.id)!.clips.length;
    const mid = vox.clips[0] ? vox.clips[0].start + Math.floor(vox.clips[0].length / 2) : 48000;
    await backend.edit({ op: "split_at", tracks: [vox.id], frame: mid }, "Split");
    await sleep(300);
    const split = (ev as ProjectEvent | null)?.project.tracks.find((t) => t.id === vox.id)?.clips.length ?? 0;
    check("split a clip (edit)", split === clipsBefore + 1, `${clipsBefore} → ${split} clips`);
    await backend.beginGesture("Mix bus");
    for (const v of [-1, -2, -3]) await backend.gestureEdit({ op: "set_mix_bus", fader_db: v });
    await backend.endGesture(false);
    await sleep(300);
    const busDb = (ev as ProjectEvent | null)?.project.mix_bus.fader_db;
    check("mix bus gesture (gesture_edit)", busDb === -3, `bus ${busDb}`);
    await backend.setLaneVisible(vox.id, "pan", true);
    await sleep(300);
    const lane = (ev as ProjectEvent | null)?.project.tracks.find((t) => t.id === vox.id)?.automation.some((l) => l.param === "pan" && l.visible);
    check("show an automation lane (view)", !!lane, `${lane}`);
    await backend.undo("mix");
    await backend.undo("mix");
    await sleep(300);
    const back = (ev as ProjectEvent | null)?.project.tracks.find((t) => t.id === vox.id)?.clips.length ?? 0;
    check("undo the edits", back === clipsBefore, `${back} clips`);
    // F7: the assistant in the real app (offline demo forced by the self-test): auto mix with the real loop and
    // engine, A/B, apply as one step with the call's id, the call logged; snapshots.
    let status: AiStatus | null = null;
    const offA = backend.onAiStatus((a) => (status = a));
    try {
      await backend.aiStart(null, null);
      const t0ai = Date.now();
      while (Date.now() - t0ai < 120_000 && !(status as AiStatus | null)?.proposal) await sleep(200);
      const prop = (status as AiStatus | null)?.proposal ?? null;
      check("auto mix proposal (real loop, offline demo)", !!prop && prop.diff.length > 0, prop ? `${prop.diff.length} tracks, ${prop.iterations.length} passes, distance ${prop.after?.distance_db?.toFixed(2)} dB, ${((Date.now() - t0ai) / 1000).toFixed(1)} s` : "none");
      await backend.aiListen("proposal");
      await sleep(300);
      check("A/B on the proposal", (status as AiStatus | null)?.listening === "proposal", `${(status as AiStatus | null)?.listening}`);
      // F17: the creative ideas (AI-27): listened to one at a time, the first ticked and applied with the plan.
      const ideas = prop?.ideas ?? [];
      if (ideas.length) {
        await backend.aiListen(`idea:${ideas[0].id}`);
        await sleep(300);
        check("listen to one idea (AI-27)", (status as AiStatus | null)?.listening === `idea:${ideas[0].id}`, `${ideas.map((i) => i.name).join(", ")}`);
        await backend.aiSetIdeas([ideas[0].id]);
        await sleep(300);
      } else {
        check("creative ideas proposed (AI-26)", false, "none on this project");
      }
      const label = await backend.aiApply(null, true);
      await sleep(400);
      if (ideas.length) check("idea applied in the same step (AI-27)", label.includes(ideas[0].name), label);
      const [entries] = await backend.history("mix");
      const lastStep = entries.filter((e) => !e.undone).at(-1);
      check("proposal applied as one step", !!lastStep?.ai_call && lastStep.label === label, `${label}`);
      const calls = (ev as ProjectEvent | null)?.project.ai.calls ?? [];
      check("call logged", calls.some((c) => c.outcome === "applied"), `${calls.length} calls`);
      // P-011: what the proposal creates is in the project and in the engine after the application (AI-17, AI-20),
      // and the new reverbs and the compressor's mix work in the real graph (FX-5, FX-3).
      // A return of an idea left unticked is not created (AI-27).
      const made = (prop?.new_elements ?? []).filter((e) => e.idea === null || e.idea === ideas[0]?.id);
      const now = await backend.getProject();
      check(
        "new returns and groups created with the proposal (AI-17)",
        made.every((e) => now.tracks.some((t) => t.id === e.id && t.kind === e.kind)),
        made.length ? made.map((e) => `${e.kind} ${e.name}`).join(", ") : "the plan created nothing on this project",
      );
      const rev = now.tracks.find((t) => t.ret?.type === "reverb");
      if (rev && rev.ret?.type === "reverb") {
        const other = rev.ret.algorithm === "plate" ? "hall" : "plate";
        // Back to the start, still playing: the auto mix may have run past the end of a short song (A-104).
        await backend.locate(0);
        await backend.edit({ op: "set_reverb_algorithm", track: rev.id, algorithm: other }, "Algorithm");
        await sleep(600);
        const got = (await backend.getProject()).tracks.find((t) => t.id === rev.id)?.ret;
        const busAfter = (last as MeterSnapshot | null)?.bus[0] ?? -200;
        check("reverb algorithm changed while playing (FX-5)", got?.type === "reverb" && got.algorithm === other && busAfter > -60, `${rev.name}: ${other}, bus ${busAfter.toFixed(1)} dBFS`);
        await backend.undo("mix");
        await sleep(300);
      }
      await backend.snapshotSave("Self-test");
      await sleep(300);
      const snap = (ev as ProjectEvent | null)?.project.snapshots.at(-1);
      check("snapshot saved", snap?.name === "Self-test", `${snap?.id}`);
      if (snap) {
        await backend.aiListen(`snapshot:${snap.id}`);
        await sleep(300);
        check("A/B on a snapshot", (status as AiStatus | null)?.listening === `snapshot:${snap.id}`, "");
        await backend.aiListen("current");
      }
      await backend.undo("mix");
    } finally {
      offA();
    }
    // F8: the master phase with the real engine and export.
    let ms: MasterStatus | null = null;
    // The steps the preparation reported (overlay 9d), and their last states.
    const prepSeen = new Map<string, string>();
    const offMs = backend.onMasterStatus((m) => {
      ms = m;
      for (const st of m.prep?.steps ?? []) prepSeen.set(st.kind, st.state + (st.source ? `/${st.source}` : ""));
      if (m.retarget) rtSeen.push([m.retarget.from_lufs, m.measures?.lufs ?? 0]);
    });
    // The waits for a new target the app reported (overlay 9v): the target each started from, and the loudness
    // on screen meanwhile.
    const rtSeen: [number, number][] = [];
    const wait = async (ok: () => boolean, ms_: number) => {
      const t = Date.now();
      while (Date.now() - t < ms_ && !ok()) await sleep(150);
      return ok();
    };
    try {
      await backend.bounceStart();
      const bounced = await wait(() => !!(ms as MasterStatus | null)?.bounced && !(ms as MasterStatus | null)?.job, 120_000);
      const b = (ms as MasterStatus | null)?.bounced;
      check("bounce (MDN-1)", bounced && !!b && b.true_peak_db <= -3 + 1e-6, b ? `mixdown ${b.number}: ${b.lufs?.toFixed(1)} LUFS, ${b.true_peak_db.toFixed(2)} dBTP, trim ${b.trim_db} dB` : "none");
      const tIn = performance.now();
      await backend.setPhase(true);
      const inMs = performance.now() - tIn;
      const measured = await wait(() => !!(ms as MasterStatus | null)?.measures && !(ms as MasterStatus | null)?.analysing, 120_000);
      const readyMs = performance.now() - tIn;
      const m = (ms as MasterStatus | null)?.measures;
      check(
        "master preparation reported its steps and ended (9d)",
        prepSeen.has("measure_mixdown") && prepSeen.has("loudness") && prepSeen.get("read_mixdown") === "done/bounce" && !(ms as MasterStatus | null)?.prep,
        [...prepSeen].map(([k, v]) => `${k}: ${v}`).join(", "),
      );
      check("master phase, loudness on target (MAS-2)", measured && !!m && Math.abs((m.lufs ?? 0) + 14) < 0.7 && m.true_peak_db <= -1 + 1e-6, `${m ? `${m.lufs?.toFixed(2)} LUFS, ${m.true_peak_db.toFixed(2)} dBTP` : "none"} · switch ${inMs.toFixed(0)} ms, measured after ${readyMs.toFixed(0)} ms`);
      // From the start: after the auto mix the playhead may be past the end of a short song (A-104).
      await backend.locate(0);
      await backend.play();
      await sleep(800);
      const mm = last as MeterSnapshot | null;
      check("master plays through the chain", (mm?.bus[0] ?? -200) > -60, `bus ${mm?.bus[0]?.toFixed(1)} dBFS, limiter ${mm?.masterGr?.[1]?.toFixed(1)} dB`);
      await backend.masterListen("mixdown");
      await sleep(300);
      check("A/B on the mixdown (MAS-5)", (ms as MasterStatus | null)?.listening === "mixdown", "");
      // Design 9l–9n: the mixdown at its real level by default, then at the master's loudness (lowered with the
      // rest by the trim). The same passage twice: the short-term loudness moves by the gain the Listening section
      // shows.
      {
        const rate0 = (await backend.getProject()).sample_rate ?? 48000;
        const momentary = async () => {
          // The short-term loudness (3 s): the momentary one swings with every hit of a synthetic beat. Read once
          // 3.5 s have played from the locate; again from the start if the meter was reset meanwhile (the engine
          // reloads when the master's measures arrive).
          const at = Math.round(5 * rate0);
          let m: MeterSnapshot | null = null;
          for (let attempt = 0; attempt < 3; attempt++) {
            // A locate while playing, as a scrub does (a pause right before a play can be overtaken by it).
            if (!(last as MeterSnapshot | null)?.playing) await backend.play();
            await backend.locate(at);
            // First the snapshot after the locate (the last one may still be from before the pause).
            await wait(() => {
              const x = last as MeterSnapshot | null;
              return !!x?.playing && x.frame >= at && x.frame < at + rate0;
            }, 5_000);
            await wait(() => {
              const x = last as MeterSnapshot | null;
              return !!x?.playing && x.frame > at + 3.5 * rate0;
            }, 10_000);
            m = last as MeterSnapshot | null;
            if ((m?.bus[6] ?? -200) > -120) break;
          }
          dbg.push(`at ${((m?.frame ?? 0) / rate0).toFixed(1)} s, M ${m?.bus[5]?.toFixed(1)}, S ${m?.bus[6]?.toFixed(1)}`);
          return m?.bus[6] ?? -200;
        };
        const dbg: string[] = [];
        const s0 = ms as MasterStatus | null;
        const real = await momentary();
        await backend.masterListenSame(true);
        await sleep(200);
        const same = await momentary();
        const s1 = ms as MasterStatus | null;
        const want = (s1?.measures?.lufs ?? 0) - (s1?.mixdown_measures?.lufs ?? 0) - (s1?.ab_trim_db ?? 0);
        check(
          "listening at the real level by default, the same loudness on demand (MAS-5, 9l–9n)",
          s0?.listen_same === false && s1?.listen_same === true && Math.abs(same - real - want) < 1.0,
          `mixdown short-term ${real.toFixed(1)} → ${same.toFixed(1)} LUFS, expected ${want >= 0 ? "+" : ""}${want.toFixed(1)} dB (trim ${(s1?.ab_trim_db ?? 0).toFixed(1)} dB; ${dbg.join(", ")})`,
        );
        await backend.masterListenSame(false);
      }
      await backend.masterListen("master");
      // 9v: a new target is reported as a wait of its own until the level is in place; going back to a target
      // already measured is not.
      {
        const before = structuredClone((await backend.getProject()).master);
        const next = structuredClone(before);
        next.target = { preset: "custom", lufs: before.target.lufs + 3, true_peak_db: before.target.true_peak_db };
        const stale = (ms as MasterStatus | null)?.measures?.lufs ?? 0;
        await backend.edit({ op: "set_master", master: next }, "Destination custom");
        const reached = await wait(() => !(ms as MasterStatus | null)?.analysing && !(ms as MasterStatus | null)?.retarget, 120_000);
        const got = (ms as MasterStatus | null)?.measures?.lufs ?? 0;
        const waits = rtSeen.length;
        await backend.undo("master");
        const back = await wait(() => !(ms as MasterStatus | null)?.analysing, 120_000);
        const again = (ms as MasterStatus | null)?.measures?.lufs ?? 0;
        check(
          "a new target waits until its level is in place, a measured one comes back at once (MAS-2, 9v)",
          reached && back && waits > 0 && rtSeen.slice(0, waits).every(([from, shown]) => from === before.target.lufs && Math.abs(shown - stale) < 1e-9) && Math.abs(got - next.target.lufs) < 0.7 && Math.abs(again - before.target.lufs) < 0.7 && !(ms as MasterStatus | null)?.retarget,
          `${stale.toFixed(2)} → ${got.toFixed(2)} LUFS for ${next.target.lufs}, ${waits} reports from ${rtSeen[0]?.[0]}; undone: ${again.toFixed(2)} LUFS`,
        );
      }
      await backend.masterAiStart(null);
      const proposed = await wait(() => !!(ms as MasterStatus | null)?.proposal, 120_000);
      check("master AI proposal measured (MAS-6, A-033)", proposed && !!(ms as MasterStatus | null)?.proposal?.after, `${(ms as MasterStatus | null)?.proposal?.diff.rows.length} changes`);
      if (proposed) await backend.masterAiApply(null);
      const p0 = await backend.getProject();
      const master = structuredClone(p0.master);
      master.export = { ...master.export, wav: true, flac: true, aac: true, bit_depth: 16, sample_rate: 44100, folder: `${project}/../exports` };
      master.metadata = { ...master.metadata, title: "Self-test", isrc: "IT-ABC-26-00001" };
      await backend.edit({ op: "set_master", master }, "Export settings");
      await backend.exportStart();
      const exported = await wait(() => !!(ms as MasterStatus | null)?.last_export && !(ms as MasterStatus | null)?.job, 180_000);
      const rep = (ms as MasterStatus | null)?.last_export ?? [];
      check("export WAV, FLAC, AAC under the ceiling (MAS-3, MAS-7, MAS-10)", exported && rep.length === 3 && rep.every((r) => r.true_peak_db <= -1 + 1e-6), rep.map((r) => `${r.format}: ${r.lufs?.toFixed(1)} LUFS ${r.true_peak_db.toFixed(2)} dBTP`).join("; "));
      // MAS-5 with a reference in another format and rate (the AAC just written, 44.1 kHz, in a 48 kHz project).
      const m4a = rep.find((r) => r.format.startsWith("AAC"))?.path;
      if (m4a) {
        await backend.addReference(m4a);
        const ready = await wait(() => ((ms as MasterStatus | null)?.references.length ?? 0) > 0 && !(ms as MasterStatus | null)?.analysing, 120_000);
        const ref = (ms as MasterStatus | null)?.references[0];
        if (ready && ref) {
          await backend.masterListen(`reference:${ref.id}`);
          // Five seconds in: the transport may have run past the end during the export, and a real song may open
          // nearly silent (the first second of the 13-stem test song peaks at −52 dBFS).
          await backend.locate(Math.round(5 * ((await backend.getProject()).sample_rate ?? 48000)));
          await backend.play();
          await sleep(900);
          const rm = last as MeterSnapshot | null;
          check("A/B on an AAC reference at another rate (MAS-5)", (ms as MasterStatus | null)?.listening === `reference:${ref.id}` && (rm?.bus[0] ?? -200) > -60, `bus ${rm?.bus[0]?.toFixed(1)} dBFS, A/B trim ${(ms as MasterStatus | null)?.ab_trim_db.toFixed(1)} dB`);
          await backend.masterListen("master");
        } else check("A/B on an AAC reference at another rate (MAS-5)", false, "reference not measured");
      }
      await backend.setPhase(false);
      check("back to Mix (MDN-3)", (ms as MasterStatus | null)?.phase === "mix", "");
      // A change that does not touch the sound (a name) keeps the mixdown current; the master opens again as it was,
      // without a new bounce. A real mix change makes it out of date, and the master still opens (MDN-2).
      const pm = await backend.getProject();
      const tr = pm.tracks.find((t) => t.kind === "audio")!;
      await backend.edit({ op: "rename_track", track: tr.id, name: `${tr.name} (renamed)`, color: null }, "Rename");
      const st1 = await backend.masterStatus();
      // Time to a drawn workspace: the switch, then the first frame where the other phase's main panel exists.
      const drawn = async (sel: string, t: number) => {
        while (!document.querySelector(sel) && performance.now() - t < 20_000) await new Promise((r) => requestAnimationFrame(() => r(null)));
        await new Promise((r) => requestAnimationFrame(() => r(null)));
        return performance.now() - t;
      };
      const tB = performance.now();
      await backend.setPhase(true);
      const backMs = performance.now() - tB;
      const paintMaster = await drawn('[aria-label="Master chain"]', tB);
      const tW = performance.now();
      while (!(await backend.masterStatus()).mixdown_measures && performance.now() - tW < 60_000) await sleep(100);
      const waveMs = performance.now() - tB;
      const back = await backend.masterStatus();
      check("rename keeps the mixdown current; Master reopens without a bounce", !st1.stale && back.phase === "master" && (await backend.getProject()).master.mixdown?.number === pm.master.mixdown?.number, `stale ${st1.stale}, phase ${back.phase}, mixdown ${pm.master.mixdown?.number} · Mix→Master ${backMs.toFixed(0)} ms, drawn ${paintMaster.toFixed(0)} ms, waveform data ${waveMs.toFixed(0)} ms`);
      const tM = performance.now();
      await backend.setPhase(false);
      const mixMs = performance.now() - tM;
      check("Master→Mix switch", true, `${mixMs.toFixed(0)} ms, drawn ${(await drawn('[aria-label="Timeline"]', tM)).toFixed(0)} ms`);
      await backend.edit({ op: "set_mix_bus", fader_db: -2 }, "Bus");
      const st2 = await backend.masterStatus();
      await backend.setPhase(true);
      check("a mix change: mixdown out of date, Master still opens (MDN-2)", st2.stale && (await backend.masterStatus()).phase === "master", `stale ${st2.stale}`);
      await backend.setPhase(false);
      await backend.undo("mix");
      await backend.undo("mix");
      // A project reopened with its master already made: what Master waits for the first time in a session.
      await backend.saveProject();
      await backend.closeProject();
      await backend.openProject(project);
      const tR = performance.now();
      await backend.setPhase(true);
      const sw = performance.now() - tR;
      let st = await backend.masterStatus();
      while (!st.mixdown_measures && performance.now() - tR < 120_000) st = await sleepThen(() => backend.masterStatus());
      const wave = performance.now() - tR;
      while ((st.analysing || !st.measures) && performance.now() - tR < 180_000) st = await sleepThen(() => backend.masterStatus());
      // The measures kept in the package: no new measuring on reopening (A-050).
      check("reopened project: Master ready from the kept measures", st.phase === "master" && wave < 3000 && performance.now() - tR < 4000 && !!st.measures, `switch ${sw.toFixed(0)} ms, waveform and measures of the mixdown ${wave.toFixed(0)} ms, loudness on target ${(performance.now() - tR).toFixed(0)} ms`);
      await backend.setPhase(false);
    } catch (e) {
      check("master exception", false, String(e));
    } finally {
      offMs();
    }
    // Scrub while playing (TRN-1): locates at 30 Hz, as a drag on the playhead sends them; playback goes on from the
    // last one, with sound.
    {
      const r = (await backend.getProject()).sample_rate ?? 48000;
      await backend.locate(0);
      await backend.play();
      for (let i = 1; i <= 20; i++) {
        await backend.locate(Math.round(i * 0.25 * r));
        await sleep(33);
      }
      await sleep(400);
      const ts = await backend.transportState();
      const busNow = (last as MeterSnapshot | null)?.bus[0] ?? -200;
      check("scrub while playing (TRN-1)", ts.playing && ts.frame >= 5 * r && ts.frame < 7 * r && busNow > -60, `at ${(ts.frame / r).toFixed(2)} s after scrubbing to 5 s, bus ${busNow.toFixed(1)} dBFS`);
    }
    await backend.stop();
    await sleep(300);
    const t2 = await backend.transportState();
    check("stop", !t2.playing, `frame ${t2.frame}`);
    check("cpu load", t1.load < 0.5, `load ${(t1.load * 100).toFixed(1)}%`);
    // The new-project screen (design 2a–2c, PRJ-3…PRJ-6), as the author uses it: files chosen before the project
    // exists, then the project created and the files imported; then "Audio track from file…" on the open project.
    try {
      const src = (await backend.getProject()).media.slice(0, 3).map((m) => `${project}/${m.path}`);
      await backend.closeProject();
      const first = await backend.importCandidates(src.slice(0, 2), false, null);
      const more = await backend.importCandidates(src.slice(2), false, first[0]?.sample_rate ?? null);
      check("new project: files listed before it exists (PRJ-3)", [...first, ...more].every((c) => !c.error && !c.rate_mismatch) && first.length + more.length === 3, [...first, ...more].map((c) => `${c.name} ${c.role} ${c.sample_rate}`).join(", "));
      const parent = project.replace(/\/[^/]+$/, "");
      await backend.createProject(parent, "Fresh");
      await backend.importFiles([...first, ...more].map((c) => ({ path: c.path, role: c.role, name: c.name, convert: false })));
      const fresh = await backend.getProject();
      check("new project created with its tracks (PRJ-4)", fresh.name === "Fresh" && fresh.tracks.filter((t) => t.kind === "audio").length === 3, `${fresh.tracks.filter((t) => t.kind === "audio").length} audio tracks at ${fresh.sample_rate} Hz`);
      const add = await backend.importCandidates(src.slice(0, 1), true, null);
      await backend.importFiles(add.map((c) => ({ path: c.path, role: c.role, name: `${c.name} 2`, convert: false })));
      const after = await backend.getProject();
      check("audio track added to the open project", after.tracks.filter((t) => t.kind === "audio").length === 4, `${after.tracks.filter((t) => t.kind === "audio").length} audio tracks`);
      // References chosen on that screen are added right after the import (PRJ-7); the settings rename the project.
      await backend.addReference(src[0]);
      await backend.edit({ op: "rename_project", name: "Fresh renamed" }, "Rename project");
      const named = await backend.getProject();
      check("reference added to the new project; project renamed (PRJ-7)", named.references.length === 1 && named.name === "Fresh renamed", `${named.references.map((r) => r.media.original_name).join(", ")} · ${named.name}`);
      // Tracks dragged in the timeline: a group from two of them, a third dragged in, then one out before the group.
      const audio = named.tracks.filter((t) => t.kind === "audio").map((t) => t.id);
      await backend.edit({ op: "add_group", name: "Group", members: audio.slice(0, 2) }, "Group 2 tracks");
      const g = (await backend.getProject()).tracks.find((t) => t.kind === "group")!;
      await backend.edit({ op: "place_tracks", tracks: [audio[2]], before: null, output: { kind: "group", id: g.id } }, "Move into Group");
      await backend.edit({ op: "place_tracks", tracks: [audio[0]], before: g.id, output: { kind: "mix_bus" } }, "Move out");
      const moved = await backend.getProject();
      const out = (id: string) => moved.tracks.find((t) => t.id === id)!.output;
      const idx = (id: string) => moved.tracks.findIndex((t) => t.id === id);
      check(
        "tracks moved into and out of a group (TRK-1, TRK-3)",
        out(audio[2]).kind === "group" && out(audio[1]).kind === "group" && out(audio[0]).kind === "mix_bus" && idx(audio[0]) < idx(g.id),
        moved.tracks.map((t) => `${t.name}→${t.output.kind === "group" ? "group" : "mix"}`).join(", "),
      );
      // Save As (PRJ-2): the copy becomes the open project, the original stays.
      await backend.saveProjectAs(parent, "Fresh copy");
      const copy = await backend.getProject();
      check("save as: the copy is the open project (PRJ-2)", copy.name === "Fresh copy" && copy.tracks.length === moved.tracks.length, `${copy.name}, ${copy.tracks.length} tracks`);
      await backend.play();
      await sleep(600);
      check("the moved routing plays", ((last as MeterSnapshot | null)?.bus[0] ?? -200) > -60, `bus ${(last as MeterSnapshot | null)?.bus[0]?.toFixed(1)} dBFS`);
      await backend.stop();
    } catch (e) {
      check("new project flow", false, String(e));
    }
    // P-020: the guided tour on the real core: the demo song built and synthesised, played, the offline demo's
    // proposal applied, a mixdown and the master, then nothing left open (PRJ-9, UI-17 → UI-19).
    try {
      const info = await backend.tourInfo();
      check("tour: no automatic start in the self-test (UI-17)", !info.auto, JSON.stringify(info));
      const recentBefore = (await backend.recentProjects()).length;
      const t0 = performance.now();
      await backend.tourStart();
      const opened = performance.now() - t0;
      const demo = await backend.getProject();
      check(
        "tour: demo song built and open, marked as the demo (PRJ-9)",
        demo.name === "Demo song" && (ev as ProjectEvent | null)?.demo === true && demo.sections.length === 3 && demo.references.length === 0,
        `${demo.tracks.filter((t) => t.kind === "audio").length} tracks · ${demo.sections.map((s) => s.name).join(", ")}`,
      );
      check("tour: open and playable within 3 s (PRJ-9)", opened < 3000, `${opened.toFixed(0)} ms`);
      await backend.play();
      await sleep(1500);
      const tbus = (last as MeterSnapshot | null)?.bus[0] ?? -200;
      check("tour: the synthesised song plays", tbus > -60, `bus peak ${tbus.toFixed(1)} dBFS`);
      await backend.stop();
      let saveRefused = false;
      try {
        await backend.saveProject();
      } catch {
        saveRefused = true;
      }
      check("tour: the demo is never saved (PRJ-9)", saveRefused);
      const before = await backend.aiStatus();
      const t1 = performance.now();
      let ready = await backend.tourProposal();
      while (!ready && performance.now() - t1 < 90_000) {
        await sleep(300);
        ready = !!(await backend.aiStatus()).proposal;
      }
      const st = await backend.aiStatus();
      const rec = st.proposal?.record;
      check("tour: proposal of the offline demo, no cost (PRJ-9, AI-9)", !!rec && rec.decider === "offline-demo" && rec.cost_usd === 0, `${rec?.decider} · $${rec?.cost_usd} · ${((performance.now() - t1) / 1000).toFixed(1)} s`);
      await backend.aiApply(null, true);
      const after = await backend.aiStatus();
      check("tour: nothing in the cost register of all projects (PRJ-9)", after.total_calls === before.total_calls && after.total_usd === before.total_usd, `${after.total_calls} calls`);
      await backend.bounceStart();
      const t2 = performance.now();
      while (!(await backend.getProject()).master.mixdown && performance.now() - t2 < 90_000) await sleep(300);
      await backend.setPhase(true);
      let mst = await backend.masterStatus();
      while ((mst.prep || mst.phase !== "master") && performance.now() - t2 < 120_000) {
        await sleep(300);
        mst = await backend.masterStatus();
      }
      check("tour: mixdown in the background and the master ready", mst.phase === "master" && !mst.prep && !!mst.mixdown_measures, `${((performance.now() - t2) / 1000).toFixed(1)} s`);
      let exportRefused = false;
      try {
        await backend.exportStart();
      } catch {
        exportRefused = true;
      }
      check("tour: nothing is exported (PRJ-9)", exportRefused);
      await backend.tourEnd();
      let closed = false;
      try {
        await backend.getProject();
      } catch {
        closed = true;
      }
      const recent = await backend.recentProjects();
      check("tour: demo closed and out of the recent projects (PRJ-9, UI-19)", closed && recent.length === recentBefore && !recent.some((r) => r.name === "Demo song"), `${recent.length} recent`);
    } catch (e) {
      check("tour flow", false, String(e));
    }
    // NFR-3: process start to the first drawn UI. Under `tauri dev` the UI comes from Vite and the build is a debug
    // one: the packaged app is measured the same way in F10.
    const cold = await backend.uiReady();
    check("cold start (NFR-3)", cold > 0 && cold < 2000, `${cold} ms`);
  } catch (e) {
    check("exception", false, String(e));
  } finally {
    offM();
    offP();
  }
  await invoke("selftest_report", { report: JSON.stringify({ ok: steps.every((s) => s.ok), steps }, null, 2) });
}
