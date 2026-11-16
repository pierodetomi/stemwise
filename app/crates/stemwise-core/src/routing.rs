//! Structured routing (TRK-3, TRK-5, FX-3): outputs to the mix bus or a group, post-fader sends to returns,
//! sidechain inputs. The graph must stay acyclic; the processing order is computed here, outside the audio
//! thread (FOUNDATION §5.4).

use std::collections::HashMap;

use crate::model::{MAX_GROUP_DEPTH, Output, Project, TrackKind};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RoutingError {
    #[error("track `{0}` does not exist")]
    MissingTrack(String),
    #[error("`{from}` cannot output to `{to}`: only groups take inputs")]
    NotAGroup { from: String, to: String },
    #[error("`{from}` cannot send to `{to}`: sends go to returns")]
    NotAReturn { from: String, to: String },
    #[error("`{0}` cannot have sends")]
    SendsNotAllowed(String),
    #[error("`{0}` cannot route to itself")]
    SelfRoute(String),
    #[error("sidechain source `{source_id}` of `{track}` must be an audio track or a group")]
    BadSidechain { track: String, source_id: String },
    #[error("groups nest at most {MAX_GROUP_DEPTH} levels deep (`{0}` would be deeper)")]
    TooDeep(String),
    #[error("this routing would create a feedback loop through `{0}`")]
    Cycle(String),
}

/// Checks the whole routing of a project (TRK-3).
pub fn validate(p: &Project) -> Result<(), RoutingError> {
    processing_order(p).map(|_| ())
}

/// Edges `a → b` meaning "a must be processed before b".
fn edges(p: &Project) -> Result<Vec<(usize, usize)>, RoutingError> {
    let index: HashMap<&str, usize> = p.tracks.iter().enumerate().map(|(i, t)| (t.id.as_str(), i)).collect();
    let find = |id: &str| index.get(id).copied().ok_or_else(|| RoutingError::MissingTrack(id.to_string()));
    let mut out = Vec::new();
    for (i, t) in p.tracks.iter().enumerate() {
        if let Output::Group(g) = &t.output {
            if g == &t.id {
                return Err(RoutingError::SelfRoute(t.id.clone()));
            }
            let gi = find(g)?;
            if p.tracks[gi].kind != TrackKind::Group {
                return Err(RoutingError::NotAGroup { from: t.id.clone(), to: g.clone() });
            }
            out.push((i, gi));
        }
        if !t.sends.is_empty() && t.kind == TrackKind::Return {
            return Err(RoutingError::SendsNotAllowed(t.id.clone()));
        }
        for s in &t.sends {
            let ri = find(&s.target)?;
            if p.tracks[ri].kind != TrackKind::Return {
                return Err(RoutingError::NotAReturn { from: t.id.clone(), to: s.target.clone() });
            }
            out.push((i, ri));
        }
        // Every block listening to another track (compressor, gate, dynamic EQ: FX-3, FX-13, FX-15), also in the
        // chain of a return (ducking, FX-11). Sources are audio tracks and groups.
        for (_, source) in t.chain.sidechains() {
            if *source == t.id {
                return Err(RoutingError::SelfRoute(t.id.clone()));
            }
            let si = find(source)?;
            if p.tracks[si].kind == TrackKind::Return {
                return Err(RoutingError::BadSidechain { track: t.id.clone(), source_id: source.clone() });
            }
            if !out.contains(&(si, i)) {
                out.push((si, i));
            }
        }
    }
    Ok(out)
}

/// Indices of `p.tracks` in an order where every track comes after everything it depends on (its inputs,
/// the tracks sending to it, its sidechain source). Fails on any invalid or cyclic routing.
pub fn processing_order(p: &Project) -> Result<Vec<usize>, RoutingError> {
    let edges = edges(p)?;
    let n = p.tracks.len();
    // Group depth: number of groups crossed from a track to the mix bus.
    for t in &p.tracks {
        let mut depth = 0;
        let mut cur = t;
        let mut steps = 0;
        while let Output::Group(g) = &cur.output {
            depth += 1;
            steps += 1;
            if steps > n {
                return Err(RoutingError::Cycle(t.id.clone()));
            }
            cur = p.track(g).ok_or_else(|| RoutingError::MissingTrack(g.clone()))?;
        }
        let inner = if t.kind == TrackKind::Group { depth + 1 } else { depth };
        if inner > MAX_GROUP_DEPTH {
            return Err(RoutingError::TooDeep(t.id.clone()));
        }
    }
    // Kahn's algorithm, stable on display order.
    let mut indegree = vec![0usize; n];
    let mut next: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in &edges {
        indegree[b] += 1;
        next[a].push(b);
    }
    let mut order = Vec::with_capacity(n);
    let mut ready: Vec<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    ready.reverse();
    while let Some(i) = ready.pop() {
        order.push(i);
        let mut newly = Vec::new();
        for &j in &next[i] {
            indegree[j] -= 1;
            if indegree[j] == 0 {
                newly.push(j);
            }
        }
        // Keep the lowest display index first.
        ready.extend(newly);
        ready.sort_unstable_by(|a, b| b.cmp(a));
    }
    if order.len() < n {
        let stuck = (0..n).find(|&i| indegree[i] > 0).unwrap_or(0);
        return Err(RoutingError::Cycle(p.tracks[stuck].id.clone()));
    }
    Ok(order)
}

/// Group nesting level of a track (0 = routed to the mix bus).
pub fn depth(p: &Project, id: &str) -> usize {
    let mut d = 0;
    let mut cur = p.track(id);
    while let Some(t) = cur {
        match &t.output {
            Output::Group(g) if d <= p.tracks.len() => {
                d += 1;
                cur = p.track(g);
            }
            _ => break,
        }
    }
    d
}

/// Outputs a track may choose without breaking the routing (the selector of design 7c).
pub fn allowed_outputs(p: &Project, id: &str) -> Vec<Output> {
    let mut out = vec![Output::MixBus];
    for g in p.tracks.iter().filter(|t| t.kind == TrackKind::Group && t.id != id) {
        let mut q = p.clone();
        if let Some(t) = q.track_mut(id) {
            t.output = Output::Group(g.id.clone());
            if validate(&q).is_ok() {
                out.push(Output::Group(g.id.clone()));
            }
        }
    }
    out
}

/// Sidechain sources a track may choose (FX-3, design 7d): the same for every block of its chain.
pub fn allowed_sidechains(p: &Project, id: &str) -> Vec<String> {
    use crate::fx::{Fx, FxBlock, FxKind};
    let mut out = Vec::new();
    for s in p.tracks.iter().filter(|t| t.kind != TrackKind::Return && t.id != id) {
        let mut q = p.clone();
        if let Some(t) = q.track_mut(id) {
            let mut probe = Fx::new(FxKind::Gate);
            if let Fx::Gate(g) = &mut probe {
                g.sidechain = Some(s.id.clone());
            }
            t.chain.blocks.push(FxBlock { id: "gate9".into(), fx: probe });
            if validate(&q).is_ok() {
                out.push(s.id.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Send, Sidechain, Track};
    use proptest::prelude::*;

    fn project() -> Project {
        let mut p = Project::new("routing");
        for (id, kind) in [("t1", 0), ("t2", 0), ("g1", 1), ("g2", 1), ("g3", 1), ("g4", 1), ("g5", 1)] {
            let t =
                if kind == 0 { Track::new_audio(id.into(), id, None, 1, 1) } else { Track::new_group(id.into(), id) };
            p.tracks.push(t);
        }
        p
    }

    fn out(p: &mut Project, id: &str, to: &str) {
        p.track_mut(id).unwrap().output = Output::Group(to.into());
    }

    /// req: TRK-3
    #[test]
    fn rejects_cycles_between_groups() {
        let mut p = project();
        out(&mut p, "t1", "g1");
        out(&mut p, "g1", "g2");
        assert!(validate(&p).is_ok());
        out(&mut p, "g2", "g1");
        assert!(matches!(validate(&p), Err(RoutingError::Cycle(_))));
    }

    /// req: TRK-3
    #[test]
    fn nests_groups_up_to_four_levels() {
        let mut p = project();
        out(&mut p, "t1", "g1");
        out(&mut p, "g1", "g2");
        out(&mut p, "g2", "g3");
        out(&mut p, "g3", "g4");
        assert!(validate(&p).is_ok());
        assert_eq!(depth(&p, "t1"), 4);
        out(&mut p, "g4", "g5");
        assert!(matches!(validate(&p), Err(RoutingError::TooDeep(_))));
    }

    /// req: TRK-3, TRK-5, FX-3
    #[test]
    fn rejects_invalid_targets_and_sidechain_loops() {
        let mut p = project();
        out(&mut p, "t1", "t2");
        assert!(matches!(validate(&p), Err(RoutingError::NotAGroup { .. })));
        let mut p = project();
        p.track_mut("t1").unwrap().sends.push(Send { target: "g1".into(), level_db: -10.0 });
        assert!(matches!(validate(&p), Err(RoutingError::NotAReturn { .. })));
        // t1 → g1, and g1's compressor listens to t1: fine (t1 is processed first).
        let mut p = project();
        out(&mut p, "t1", "g1");
        p.track_mut("g1").unwrap().chain.comp_mut().sidechain = Some(Sidechain { source: "t1".into(), hpf_hz: 100.0 });
        assert!(validate(&p).is_ok());
        // t1's compressor listening to g1, which t1 feeds: loop.
        let mut p = project();
        out(&mut p, "t1", "g1");
        p.track_mut("t1").unwrap().chain.comp_mut().sidechain = Some(Sidechain { source: "g1".into(), hpf_hz: 100.0 });
        assert!(matches!(validate(&p), Err(RoutingError::Cycle(_))));
        assert!(!allowed_sidechains(&p, "t1").contains(&"g1".to_string()));
        // A return routed into a group that sends to the same return: loop.
        let mut p = project();
        let r1 = p.tracks[0].id.clone();
        out(&mut p, &r1, "g1");
        p.track_mut("g1").unwrap().sends.push(Send { target: r1.clone(), level_db: -10.0 });
        assert!(matches!(validate(&p), Err(RoutingError::Cycle(_))));
    }

    /// req: TRK-3
    #[test]
    fn allowed_outputs_exclude_loops() {
        let mut p = project();
        out(&mut p, "g1", "g2");
        let allowed = allowed_outputs(&p, "g2");
        assert!(allowed.contains(&Output::MixBus));
        assert!(!allowed.contains(&Output::Group("g1".into())));
        assert!(allowed.contains(&Output::Group("g3".into())));
    }

    proptest! {
        /// req: TRK-3
        #[test]
        fn random_routings_are_either_rejected_or_ordered(
            outs in proptest::collection::vec(0usize..8, 7),
            sc in proptest::collection::vec(0usize..8, 7),
        ) {
            let mut p = project();
            let ids: Vec<String> = p.tracks.iter().map(|t| t.id.clone()).collect();
            let groups = ["g1", "g2", "g3", "g4", "g5"];
            for (k, id) in ids.iter().enumerate().skip(2) {
                let o = outs[k - 2];
                p.track_mut(id).unwrap().output = if o < 5 { Output::Group(groups[o].into()) } else { Output::MixBus };
                let s = sc[k - 2];
                if s < ids.len() && !p.tracks[s].is_return() && p.tracks[k].kind != TrackKind::Return {
                    p.track_mut(id).unwrap().chain.comp_mut().sidechain = Some(Sidechain { source: ids[s].clone(), hpf_hz: 100.0 });
                }
            }
            if let Ok(order) = processing_order(&p) {
                let pos: HashMap<usize, usize> = order.iter().enumerate().map(|(k, &i)| (i, k)).collect();
                for (a, b) in edges(&p).unwrap() {
                    prop_assert!(pos[&a] < pos[&b]);
                }
                for t in &p.tracks {
                    prop_assert!(depth(&p, &t.id) <= MAX_GROUP_DEPTH);
                }
            }
        }
    }
}
