//! Transactions, undo/redo and continuous gestures (HIS-1, HIS-2, HIS-3, HIS-5).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::command::{Command, CommandError};
use crate::model::Project;
use crate::routing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Mix,
    Master,
}

/// One step of the history: commands applied together, with their inverses.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub label: String,
    pub commands: Vec<Command>,
    /// Inverses in application order; undo applies them in reverse.
    pub inverse: Vec<Command>,
    /// AI call the step comes from (HIS-3).
    pub ai_call: Option<String>,
}

/// A readable entry of the history panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct HistoryEntry {
    pub index: u32,
    pub label: String,
    pub ai_call: Option<String>,
    /// True for undone steps that can still be redone.
    pub undone: bool,
}

#[derive(Debug, Clone, Default)]
pub struct History {
    done: Vec<Transaction>,
    undone: Vec<Transaction>,
}

impl History {
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Steps oldest first: the applied ones, then the undone ones (most recently undone last).
    pub fn entries(&self) -> Vec<HistoryEntry> {
        let applied = self.done.iter().map(|t| (t, false));
        let redoable = self.undone.iter().rev().map(|t| (t, true));
        applied
            .chain(redoable)
            .enumerate()
            .map(|(i, (t, undone))| HistoryEntry {
                index: i as u32 + 1,
                label: t.label.clone(),
                ai_call: t.ai_call.clone(),
                undone,
            })
            .collect()
    }

    /// Number of applied steps (the position in the history).
    pub fn position(&self) -> usize {
        self.done.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error(transparent)]
    Routing(#[from] routing::RoutingError),
    #[error("a gesture is already in progress")]
    GestureOpen,
    #[error("no gesture in progress")]
    NoGesture,
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
}

/// What changed in the document, for the engine and the UI.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Change {
    /// Something other than parameter values changed: the engine must rebuild its graph snapshot.
    pub structural: bool,
    /// Parameter values set, as (track, param, value), for the engine's parameter queue.
    pub params: Vec<(String, String, f64)>,
    /// New mix bus fader level (a parameter too: no rebuild).
    pub bus_fader: Option<f64>,
    /// Solo flags changed (applied as a mask, no rebuild).
    pub solo: bool,
    /// The master state changed (master phase: new chain parameters, no rebuild of the mix graph).
    pub master: bool,
}

impl Change {
    fn record(&mut self, c: &Command) {
        match c {
            Command::SetParam { track, param, value } => self.params.push((track.clone(), param.clone(), *value)),
            Command::SetMixBus { bus } => self.bus_fader = Some(bus.fader_db),
            Command::SetSolo { .. } => self.solo = true,
            Command::SetMaster { .. } => self.master = true,
            // The call log does not touch the sound; a new mixdown is loaded by the master phase itself.
            Command::SetAiCalls { .. }
            | Command::SetAiNotes { .. }
            | Command::SetMixdown { .. }
            | Command::SetAllowStructure { .. }
            | Command::SetAllowIdeas { .. } => {}
            _ => self.structural = true,
        }
    }

    fn merge(&mut self, other: Change) {
        self.structural |= other.structural;
        self.params.extend(other.params);
        self.bus_fader = other.bus_fader.or(self.bus_fader);
        self.solo |= other.solo;
        self.master |= other.master;
    }
}

struct Gesture {
    phase: Phase,
    tx: Transaction,
}

/// The document with its two histories (HIS-5) and the gesture in progress (HIS-2).
pub struct Document {
    pub project: Project,
    mix: History,
    master: History,
    gesture: Option<Gesture>,
    /// Incremented on every change, history or not.
    pub revision: u64,
    /// Changes since the last save.
    pub dirty: bool,
}

/// Applies commands in order; on failure, reverts those already applied.
fn apply_all(p: &mut Project, commands: &[Command]) -> Result<(Vec<Command>, Change), HistoryError> {
    let mut inverse = Vec::with_capacity(commands.len());
    let mut change = Change::default();
    for c in commands {
        match c.clone().apply(p) {
            Ok(inv) => {
                change.record(c);
                inverse.push(inv);
            }
            Err(e) => {
                for inv in inverse.into_iter().rev() {
                    inv.apply(p).expect("inverse of an applied command");
                }
                return Err(e.into());
            }
        }
    }
    if change.structural
        && let Err(e) = routing::validate(p)
    {
        for inv in inverse.into_iter().rev() {
            inv.apply(p).expect("inverse of an applied command");
        }
        return Err(e.into());
    }
    Ok((inverse, change))
}

fn revert(p: &mut Project, inverse: &[Command]) -> Change {
    let mut change = Change::default();
    for inv in inverse.iter().rev() {
        change.record(inv);
        inv.clone().apply(p).expect("inverse of an applied command");
    }
    change
}

impl Document {
    pub fn new(project: Project) -> Self {
        Document {
            project,
            mix: History::default(),
            master: History::default(),
            gesture: None,
            revision: 0,
            dirty: false,
        }
    }

    pub fn history(&self, phase: Phase) -> &History {
        match phase {
            Phase::Mix => &self.mix,
            Phase::Master => &self.master,
        }
    }

    fn history_mut(&mut self, phase: Phase) -> &mut History {
        match phase {
            Phase::Mix => &mut self.mix,
            Phase::Master => &mut self.master,
        }
    }

    fn touched(&mut self) {
        self.revision += 1;
        self.dirty = true;
    }

    /// Applies commands as one history step (HIS-1, HIS-3). Fails without changing anything.
    pub fn transact(
        &mut self,
        phase: Phase,
        label: &str,
        commands: Vec<Command>,
        ai_call: Option<String>,
    ) -> Result<Change, HistoryError> {
        if self.gesture.is_some() {
            return Err(HistoryError::GestureOpen);
        }
        if commands.is_empty() {
            return Ok(Change::default());
        }
        let (inverse, change) = apply_all(&mut self.project, &commands)?;
        let h = self.history_mut(phase);
        h.done.push(Transaction { label: label.to_string(), commands, inverse, ai_call });
        h.undone.clear();
        self.touched();
        Ok(change)
    }

    /// Applies a change that never enters the history (mute, solo, lane visibility: HIS-6).
    pub fn apply_view(&mut self, commands: Vec<Command>) -> Result<Change, HistoryError> {
        let (_, change) = apply_all(&mut self.project, &commands)?;
        self.touched();
        Ok(change)
    }

    /// Opens a continuous gesture (dragging a fader, a clip, a point): one step when it ends (HIS-2).
    pub fn begin_gesture(&mut self, phase: Phase, label: &str) -> Result<(), HistoryError> {
        if self.gesture.is_some() {
            return Err(HistoryError::GestureOpen);
        }
        let tx = Transaction { label: label.to_string(), commands: Vec::new(), inverse: Vec::new(), ai_call: None };
        self.gesture = Some(Gesture { phase, tx });
        Ok(())
    }

    pub fn gesture_open(&self) -> bool {
        self.gesture.is_some()
    }

    /// Applies commands inside the open gesture. Repeated values of the same parameter coalesce.
    pub fn gesture_apply(&mut self, commands: Vec<Command>) -> Result<Change, HistoryError> {
        if self.gesture.is_none() {
            return Err(HistoryError::NoGesture);
        }
        let (inverse, change) = apply_all(&mut self.project, &commands)?;
        let g = self.gesture.as_mut().expect("checked above");
        for (c, inv) in commands.into_iter().zip(inverse) {
            // A value already set in this gesture is updated in place (its first inverse is kept): dragging an EQ
            // handle alternates frequency and gain and still stays one compact step.
            let coalesced = match &c {
                Command::SetParam { track, param, value } => g.tx.commands.iter_mut().any(|x| match x {
                    Command::SetParam { track: lt, param: lp, value: lv } if lt == track && lp == param => {
                        *lv = *value;
                        true
                    }
                    _ => false,
                }),
                Command::SetMaster { master } => match g.tx.commands.last_mut() {
                    Some(Command::SetMaster { master: last }) => {
                        **last = (**master).clone();
                        true
                    }
                    _ => false,
                },
                Command::SetTrack { track } => match g.tx.commands.last_mut() {
                    Some(Command::SetTrack { track: last }) if track.id == last.id => {
                        **last = (**track).clone();
                        true
                    }
                    _ => false,
                },
                _ => false,
            };
            if !coalesced {
                g.tx.commands.push(c);
                g.tx.inverse.push(inv);
            }
        }
        self.revision += 1;
        Ok(change)
    }

    /// Returns the document to the state at the start of the open gesture, keeping the gesture open: a drag
    /// recomputed from its origin (an offset curve, a send level) replaces what it applied so far.
    pub fn gesture_reset(&mut self) -> Result<Change, HistoryError> {
        let g = self.gesture.as_mut().ok_or(HistoryError::NoGesture)?;
        let change = revert(&mut self.project, &g.tx.inverse);
        g.tx.commands.clear();
        g.tx.inverse.clear();
        self.revision += 1;
        Ok(change)
    }

    /// Replaces what the open gesture applied with the commands `f` computes from the gesture's starting state.
    pub fn gesture_replace<E>(
        &mut self,
        f: impl FnOnce(&mut Project) -> Result<Vec<Command>, E>,
    ) -> Result<Result<Change, E>, HistoryError> {
        let mut change = self.gesture_reset()?;
        match f(&mut self.project) {
            Ok(cmds) => {
                change.merge(self.gesture_apply(cmds)?);
                Ok(Ok(change))
            }
            Err(e) => Ok(Err(e)),
        }
    }

    /// Closes the gesture as one history step. A gesture that changed nothing leaves no step.
    pub fn end_gesture(&mut self) -> Result<Change, HistoryError> {
        let g = self.gesture.take().ok_or(HistoryError::NoGesture)?;
        let mut change = Change::default();
        if !g.tx.commands.is_empty() {
            for c in &g.tx.commands {
                change.record(c);
            }
            let h = self.history_mut(g.phase);
            h.done.push(g.tx);
            h.undone.clear();
            self.touched();
        }
        Ok(change)
    }

    /// Cancels the gesture (Esc): the document returns to its state before it, with no history step.
    pub fn cancel_gesture(&mut self) -> Result<Change, HistoryError> {
        let g = self.gesture.take().ok_or(HistoryError::NoGesture)?;
        let change = revert(&mut self.project, &g.tx.inverse);
        self.revision += 1;
        Ok(change)
    }

    pub fn undo(&mut self, phase: Phase) -> Result<(String, Change), HistoryError> {
        if self.gesture.is_some() {
            return Err(HistoryError::GestureOpen);
        }
        let tx = self.history_mut(phase).done.pop().ok_or(HistoryError::NothingToUndo)?;
        let change = revert(&mut self.project, &tx.inverse);
        let label = tx.label.clone();
        self.history_mut(phase).undone.push(tx);
        self.touched();
        Ok((label, change))
    }

    pub fn redo(&mut self, phase: Phase) -> Result<(String, Change), HistoryError> {
        if self.gesture.is_some() {
            return Err(HistoryError::GestureOpen);
        }
        let mut tx = self.history_mut(phase).undone.pop().ok_or(HistoryError::NothingToRedo)?;
        let (inverse, change) = apply_all(&mut self.project, &tx.commands)?;
        tx.inverse = inverse;
        let label = tx.label.clone();
        self.history_mut(phase).done.push(tx);
        self.touched();
        Ok((label, change))
    }

    /// Moves to a point of the history panel: `position` applied steps (HIS-1, "click to go back").
    pub fn goto(&mut self, phase: Phase, position: usize) -> Result<Change, HistoryError> {
        let mut change = Change::default();
        while self.history(phase).position() > position {
            change.merge(self.undo(phase)?.1);
        }
        while self.history(phase).position() < position && self.history(phase).can_redo() {
            change.merge(self.redo(phase)?.1);
        }
        Ok(change)
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Output, Section, Track};
    use proptest::prelude::*;

    fn doc() -> Document {
        let mut p = Project::new("h");
        for i in 1..=4 {
            p.tracks.push(Track::new_audio(format!("t{i}"), &format!("Track {i}"), None, 1, i as u8));
        }
        p.tracks.push(Track::new_group("g1".into(), "Group"));
        Document::new(p)
    }

    fn fader(t: &str, v: f64) -> Command {
        Command::SetParam { track: t.into(), param: "fader".into(), value: v }
    }

    /// req: HIS-1
    #[test]
    fn undo_and_redo_are_unlimited_and_labelled() {
        let mut d = doc();
        for i in 0..200 {
            d.transact(Phase::Mix, &format!("Step {i}"), vec![fader("t1", -(i as f64) / 10.0)], None).unwrap();
        }
        assert_eq!(d.history(Phase::Mix).entries().len(), 200);
        for _ in 0..200 {
            d.undo(Phase::Mix).unwrap();
        }
        assert_eq!(d.project.track("t1").unwrap().chain.fader_db, 0.0);
        assert!(d.undo(Phase::Mix).is_err());
        let (label, _) = d.redo(Phase::Mix).unwrap();
        assert_eq!(label, "Step 0");
        d.goto(Phase::Mix, 150).unwrap();
        assert_eq!(d.project.track("t1").unwrap().chain.fader_db, -14.9);
        let e = d.history(Phase::Mix).entries();
        assert!(e[149].label == "Step 149" && !e[149].undone && e[150].undone);
    }

    /// req: HIS-2
    #[test]
    fn a_gesture_is_one_step_and_escape_leaves_none() {
        let mut d = doc();
        d.begin_gesture(Phase::Mix, "Move Track 1 level").unwrap();
        for k in 0..50 {
            d.gesture_apply(vec![fader("t1", -(k as f64) * 0.2)]).unwrap();
        }
        assert!(d.transact(Phase::Mix, "x", vec![fader("t2", -1.0)], None).is_err());
        d.end_gesture().unwrap();
        assert_eq!(d.history(Phase::Mix).entries().len(), 1);
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project.track("t1").unwrap().chain.fader_db, 0.0);

        d.begin_gesture(Phase::Mix, "Move Track 2 level").unwrap();
        d.gesture_apply(vec![fader("t2", -9.0)]).unwrap();
        d.cancel_gesture().unwrap();
        assert_eq!(d.project.track("t2").unwrap().chain.fader_db, 0.0);
        assert_eq!(d.history(Phase::Mix).position(), 0);
    }

    /// req: HIS-2, FX-7
    #[test]
    fn a_gesture_on_two_parameters_stays_compact_and_undoes_both() {
        let mut d = doc();
        d.begin_gesture(Phase::Mix, "EQ band 2").unwrap();
        for k in 0..100 {
            let f = Command::SetParam { track: "t1".into(), param: "eq.2.freq".into(), value: 500.0 + k as f64 };
            let g = Command::SetParam { track: "t1".into(), param: "eq.2.gain".into(), value: k as f64 * 0.1 };
            d.gesture_apply(vec![f, g]).unwrap();
        }
        d.end_gesture().unwrap();
        let b = d.project.track("t1").unwrap().chain.eq().unwrap().bands[1];
        assert_eq!((b.freq_hz, (b.gain_db * 10.0).round()), (599.0, 99.0));
        d.undo(Phase::Mix).unwrap();
        assert_eq!(
            d.project.track("t1").unwrap().chain.eq().unwrap().bands[1],
            doc().project.track("t1").unwrap().chain.eq().unwrap().bands[1]
        );
        d.redo(Phase::Mix).unwrap();
        assert_eq!(d.project.track("t1").unwrap().chain.eq().unwrap().bands[1].freq_hz, 599.0);
    }

    /// req: HIS-2, AUT-3
    #[test]
    fn a_replaced_gesture_is_computed_from_its_start_and_stays_one_step() {
        let mut d = doc();
        d.begin_gesture(Phase::Mix, "Offset").unwrap();
        for k in 1..=10 {
            // Each move recomputes from the starting value: the result is start + last delta, not a running sum.
            let r = d
                .gesture_replace(|p| -> Result<Vec<Command>, ()> {
                    let v = p.track("t1").unwrap().chain.fader_db - k as f64;
                    Ok(vec![fader("t1", v)])
                })
                .unwrap();
            assert!(r.is_ok());
        }
        d.end_gesture().unwrap();
        assert_eq!(d.project.track("t1").unwrap().chain.fader_db, -10.0);
        assert_eq!(d.history(Phase::Mix).entries().len(), 1);
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project.track("t1").unwrap().chain.fader_db, 0.0);
    }

    /// req: HIS-3
    #[test]
    fn an_ai_proposal_is_a_single_transaction_with_its_call() {
        let mut d = doc();
        let cmds = (1..=4).map(|i| fader(&format!("t{i}"), -2.0)).collect();
        d.transact(Phase::Mix, "Apply auto mix", cmds, Some("call-1".into())).unwrap();
        let e = d.history(Phase::Mix).entries();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].ai_call.as_deref(), Some("call-1"));
        d.undo(Phase::Mix).unwrap();
        assert!(d.project.tracks.iter().all(|t| t.chain.fader_db == 0.0 || t.is_return()));
    }

    /// req: HIS-5
    #[test]
    fn mix_and_master_have_separate_histories() {
        let mut d = doc();
        d.transact(Phase::Mix, "Mix step", vec![fader("t1", -3.0)], None).unwrap();
        let mut m = d.project.master.clone();
        m.chain.input_gain_db = 2.0;
        d.transact(Phase::Master, "Master step", vec![Command::SetMaster { master: Box::new(m) }], None).unwrap();
        d.undo(Phase::Master).unwrap();
        assert_eq!(d.project.master.chain.input_gain_db, 0.0);
        assert_eq!(d.project.track("t1").unwrap().chain.fader_db, -3.0);
        assert!(d.undo(Phase::Master).is_err());
    }

    /// req: HIS-5, MDN-2
    #[test]
    fn master_undo_and_redo_never_move_the_mixdown() {
        let mut d = doc();
        let mut m = d.project.master.clone();
        m.chain.input_gain_db = 1.0;
        d.transact(Phase::Master, "Input", vec![Command::SetMaster { master: Box::new(m) }], None).unwrap();
        // A bounce records a new mixdown outside the histories.
        let md = crate::model::Mixdown {
            number: 2,
            path: "mixdowns/mixdown-2.wav".into(),
            mix_hash: "h".into(),
            trim_db: 0.0,
            frames: 1,
            created_ms: 0,
            lufs: None,
            true_peak_db: -3.0,
            lra: None,
        };
        d.apply_view(vec![Command::SetMixdown { mixdown: Some(Box::new(md.clone())) }]).unwrap();
        d.undo(Phase::Master).unwrap();
        assert_eq!(d.project.master.chain.input_gain_db, 0.0);
        assert_eq!(d.project.master.mixdown.as_ref(), Some(&md), "undo keeps the newest mixdown");
        d.redo(Phase::Master).unwrap();
        assert_eq!(d.project.master.chain.input_gain_db, 1.0);
        assert_eq!(d.project.master.mixdown.as_ref(), Some(&md), "redo too");
        // A stale copy of the master state (without the new mixdown) cannot move it back either.
        let mut stale = d.project.master.clone();
        stale.mixdown = None;
        stale.chain.input_gain_db = 2.0;
        d.transact(Phase::Master, "Input", vec![Command::SetMaster { master: Box::new(stale) }], None).unwrap();
        assert_eq!(d.project.master.mixdown.as_ref(), Some(&md));
    }

    /// req: HIS-6
    #[test]
    fn view_changes_do_not_enter_the_history() {
        let mut d = doc();
        d.apply_view(vec![Command::SetParam { track: "t1".into(), param: "mute".into(), value: 1.0 }]).unwrap();
        assert!(d.project.track("t1").unwrap().mute);
        assert_eq!(d.history(Phase::Mix).position(), 0);
    }

    /// req: TRK-3
    #[test]
    fn a_transaction_creating_a_cycle_is_rejected_atomically() {
        let mut d = doc();
        let mut t = d.project.track("g1").unwrap().clone();
        t.output = Output::Group("g1".into());
        let before = d.project.clone();
        assert!(
            d.transact(Phase::Mix, "Route", vec![fader("t1", -1.0), Command::SetTrack { track: Box::new(t) }], None)
                .is_err()
        );
        assert_eq!(d.project, before);
        assert_eq!(d.history(Phase::Mix).position(), 0);
    }

    fn arb_command() -> impl Strategy<Value = Command> {
        let track = prop_oneof![Just("t1"), Just("t2"), Just("t3"), Just("t4"), Just("g1"), Just("r1")];
        let param = prop_oneof![
            Just("fader"),
            Just("pan"),
            Just("trim"),
            Just("eq.2.gain"),
            Just("comp.ratio"),
            Just("bypass.comp"),
            Just("hpf.freq")
        ];
        prop_oneof![
            (track.clone(), param, -1.0f64..1.0).prop_map(|(t, p, v)| Command::SetParam {
                track: t.into(),
                param: p.into(),
                value: if p == "comp.ratio" {
                    1.0 + v.abs() * 5.0
                } else if p == "hpf.freq" {
                    100.0 + v * 50.0
                } else {
                    v
                },
            }),
            (track.clone(), 0usize..6).prop_map(|(t, i)| Command::MoveTrack { track: t.into(), index: i }),
            (track.clone(), "[a-z]{1,8}").prop_map(|(t, n)| Command::SetName { name: format!("{t}{n}") }),
            (0i64..100, 1i64..100).prop_map(|(a, l)| Command::SetSections {
                sections: vec![Section { id: "s1".into(), name: "A".into(), color: 1, start: a, end: a + l }]
            }),
            (0usize..7).prop_map(|i| Command::InsertTrack {
                index: i.min(6),
                track: Box::new(Track::new_group(format!("gx{i}"), "New")),
            }),
            track.clone().prop_map(|t| Command::RemoveTrack { track: t.into() }),
            (track, prop_oneof![Just(None), Just(Some("g1"))]).prop_map(|(t, g)| {
                let mut tr = Track::new_audio(t.into(), "Replaced", None, 2, 3);
                tr.output = g.map_or(Output::MixBus, |g| Output::Group(g.into()));
                Command::SetTrack { track: Box::new(tr) }
            }),
            (prop_oneof![Just("m1"), Just("m2"), Just("mx")], prop_oneof![Just("m1"), Just("m2"), Just("mx")])
                .prop_map(|(a, b)| Command::RelinkMedia { from: a.into(), to: b.into() }),
        ]
    }

    proptest! {
        /// req: HIS-1, HIS-2
        #[test]
        fn any_command_sequence_followed_by_undos_restores_the_document(
            batches in proptest::collection::vec(proptest::collection::vec(arb_command(), 1..4), 1..25)
        ) {
            let mut d = doc();
            let initial = d.project.clone();
            let mut steps = 0;
            for (i, batch) in batches.into_iter().enumerate() {
                if i % 3 == 0 {
                    d.begin_gesture(Phase::Mix, "gesture").unwrap();
                    if d.gesture_apply(batch).is_ok() {
                        // may still be empty if nothing applied
                    }
                    let had = d.history(Phase::Mix).position();
                    d.end_gesture().unwrap();
                    steps += d.history(Phase::Mix).position() - had;
                } else if d.transact(Phase::Mix, "step", batch, None).is_ok() {
                    steps += 1;
                }
            }
            prop_assert_eq!(d.history(Phase::Mix).position(), steps);
            for _ in 0..steps {
                d.undo(Phase::Mix).unwrap();
            }
            prop_assert_eq!(&d.project, &initial);
        }
    }
}
