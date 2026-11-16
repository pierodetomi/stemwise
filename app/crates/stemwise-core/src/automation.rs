//! Automation lanes (AUT-1 … AUT-5): evaluation and editing.
//!
//! Points are sorted by frame. Continuous parameters interpolate linearly between points and hold the first
//! and last values outside them; mute and bypass are steps (AUT-1).

use crate::command::Command;
use crate::model::{Author, AutomationLane, AutomationPoint, Project, Section};
use crate::params::{self, ParamId};

/// Value of a lane at `frame` (`None` for an empty lane).
pub fn value_at(points: &[AutomationPoint], frame: i64, stepped: bool) -> Option<f64> {
    let first = points.first()?;
    if frame <= first.frame {
        return Some(first.value);
    }
    let i = points.partition_point(|p| p.frame <= frame);
    if i >= points.len() {
        return Some(points[points.len() - 1].value);
    }
    let (a, b) = (points[i - 1], points[i]);
    if stepped || b.frame == a.frame {
        return Some(a.value);
    }
    let t = (frame - a.frame) as f64 / (b.frame - a.frame) as f64;
    Some(a.value + (b.value - a.value) * t)
}

pub fn sort_points(points: &mut [AutomationPoint]) {
    points.sort_by_key(|p| p.frame);
}

/// Ramer–Douglas–Peucker simplification of a freehand stroke (AUT-2), tolerance in value units.
pub fn simplify(points: &[AutomationPoint], tolerance: f64) -> Vec<AutomationPoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0usize, points.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let (pa, pb) = (points[a], points[b]);
        let mut worst = (0.0f64, a);
        for (k, p) in points.iter().enumerate().take(b).skip(a + 1) {
            let t = if pb.frame == pa.frame { 0.0 } else { (p.frame - pa.frame) as f64 / (pb.frame - pa.frame) as f64 };
            let d = (p.value - (pa.value + (pb.value - pa.value) * t)).abs();
            if d > worst.0 {
                worst = (d, k);
            }
        }
        if worst.0 > tolerance {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// Commands replacing a lane's points (creating the lane if needed); the lane becomes the user's.
pub fn set_lane(
    p: &Project,
    track: &str,
    param: &str,
    points: Vec<AutomationPoint>,
    author: Author,
) -> Option<Command> {
    let id: ParamId = param.parse().ok()?;
    let spec = id.spec();
    let mut t = p.track(track)?.clone();
    params::get(&t, &id)?;
    let mut points: Vec<AutomationPoint> = points
        .into_iter()
        .map(|mut pt| {
            pt.value = pt.value.clamp(spec.min, spec.max);
            if spec.stepped {
                pt.value = if pt.value >= 0.5 { 1.0 } else { 0.0 };
            }
            pt
        })
        .collect();
    sort_points(&mut points);
    match t.automation.iter_mut().find(|l| l.param == param) {
        Some(l) => {
            l.points = points;
            l.author = author;
        }
        None => t.automation.push(AutomationLane { param: param.into(), points, author, visible: true }),
    }
    Some(Command::SetTrack { track: Box::new(t) })
}

/// AUT-3: moving a control of an automated parameter while stopped shifts the whole curve by an offset.
pub fn offset_lane(p: &Project, track: &str, param: &str, delta: f64) -> Option<Command> {
    let t = p.track(track)?;
    let lane = t.lane(param)?;
    let pts = lane.points.iter().map(|pt| AutomationPoint { frame: pt.frame, value: pt.value + delta }).collect();
    set_lane(p, track, param, pts, Author::User)
}

/// Points of a lane inside `[from, to)`, with frames relative to `from` (AUT-2 copy).
pub fn copy_range(lane: &AutomationLane, from: i64, to: i64, stepped: bool) -> Vec<AutomationPoint> {
    let mut out: Vec<AutomationPoint> = Vec::new();
    if let Some(v) = value_at(&lane.points, from, stepped) {
        out.push(AutomationPoint { frame: 0, value: v });
    }
    out.extend(
        lane.points
            .iter()
            .filter(|pt| pt.frame > from && pt.frame < to)
            .map(|pt| AutomationPoint { frame: pt.frame - from, value: pt.value }),
    );
    if let Some(v) = value_at(&lane.points, to - 1, stepped) {
        out.push(AutomationPoint { frame: to - from - 1, value: v });
    }
    out
}

/// Pastes copied points at `at`, replacing what the lane had over the pasted span (AUT-2 paste).
pub fn paste_range(existing: &[AutomationPoint], clip: &[AutomationPoint], at: i64) -> Vec<AutomationPoint> {
    let Some(len) = clip.last().map(|p| p.frame) else {
        return existing.to_vec();
    };
    let mut out: Vec<AutomationPoint> =
        existing.iter().filter(|p| p.frame < at || p.frame > at + len).copied().collect();
    out.extend(clip.iter().map(|p| AutomationPoint { frame: p.frame + at, value: p.value }));
    sort_points(&mut out);
    out
}

/// Deletes the points inside `[from, to]` (AUT-2 range delete).
pub fn delete_range(existing: &[AutomationPoint], from: i64, to: i64) -> Vec<AutomationPoint> {
    existing.iter().filter(|p| p.frame < from || p.frame > to).copied().collect()
}

/// AUT-5: points for per-section values around a static base value, with a linear ramp of `ramp` frames
/// centred on each section boundary. `offsets[i]` applies to `sections[i]` (0 = base value).
pub fn section_points(sections: &[Section], base: f64, offsets: &[f64], ramp: i64) -> Vec<AutomationPoint> {
    let mut secs: Vec<(&Section, f64)> = sections.iter().zip(offsets.iter().copied()).collect();
    secs.sort_by_key(|(s, _)| s.start);
    let mut pts: Vec<AutomationPoint> = Vec::new();
    let half = ramp / 2;
    let mut prev_value = base;
    let mut prev_end: Option<i64> = None;
    for (s, off) in secs {
        let v = base + off;
        // Boundary at s.start: ramp from the previous value to this one.
        let boundary = s.start;
        if (v - prev_value).abs() > 1e-12 {
            let gap_before = prev_end.is_some_and(|e| e < boundary);
            if gap_before {
                // Back to base between non-adjacent sections.
                let e = prev_end.expect("checked");
                if (prev_value - base).abs() > 1e-12 {
                    pts.push(AutomationPoint { frame: (e - half).max(0), value: prev_value });
                    pts.push(AutomationPoint { frame: e + half, value: base });
                }
                if (v - base).abs() > 1e-12 {
                    pts.push(AutomationPoint { frame: (boundary - half).max(0), value: base });
                    pts.push(AutomationPoint { frame: boundary + half, value: v });
                }
            } else {
                pts.push(AutomationPoint { frame: (boundary - half).max(0), value: prev_value });
                pts.push(AutomationPoint { frame: boundary + half, value: v });
            }
        }
        prev_value = v;
        prev_end = Some(s.end);
    }
    if let Some(e) = prev_end
        && (prev_value - base).abs() > 1e-12
    {
        pts.push(AutomationPoint { frame: (e - half).max(0), value: prev_value });
        pts.push(AutomationPoint { frame: e + half, value: base });
    }
    if pts.is_empty() {
        return pts;
    }
    // Ensure the lane starts at the base value when the first section does not start at 0.
    if pts[0].frame > 0 && (pts[0].value - base).abs() < 1e-12 {
        pts.insert(0, AutomationPoint { frame: 0, value: base });
    }
    sort_points(&mut pts);
    pts.dedup_by(|a, b| a.frame == b.frame && (a.value - b.value).abs() < 1e-12);
    pts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Track;

    fn pt(frame: i64, value: f64) -> AutomationPoint {
        AutomationPoint { frame, value }
    }

    /// req: AUT-1
    #[test]
    fn linear_and_stepped_evaluation() {
        let pts = [pt(100, 0.0), pt(200, 1.0), pt(300, -1.0)];
        assert_eq!(value_at(&pts, 0, false), Some(0.0));
        assert_eq!(value_at(&pts, 150, false), Some(0.5));
        assert_eq!(value_at(&pts, 250, false), Some(0.0));
        assert_eq!(value_at(&pts, 999, false), Some(-1.0));
        assert_eq!(value_at(&pts, 150, true), Some(0.0));
        assert_eq!(value_at(&pts, 200, true), Some(1.0));
        assert_eq!(value_at(&[], 5, false), None);
    }

    /// req: AUT-2
    #[test]
    fn freehand_strokes_are_simplified_and_ranges_copy_paste() {
        let stroke: Vec<_> = (0..1000).map(|i| pt(i, (i as f64 / 1000.0) * 6.0)).collect();
        let s = simplify(&stroke, 0.01);
        assert_eq!(s.len(), 2, "a straight stroke keeps its ends only");
        let bumpy: Vec<_> = (0..1000).map(|i| pt(i, if i == 500 { 3.0 } else { 0.0 })).collect();
        assert!(simplify(&bumpy, 0.1).iter().any(|p| p.frame == 500));
        let lane = AutomationLane {
            param: "fader".into(),
            points: vec![pt(0, 0.0), pt(100, -10.0)],
            author: Author::User,
            visible: true,
        };
        let clip = copy_range(&lane, 0, 101, false);
        let pasted = paste_range(&[pt(0, 0.0), pt(1000, 0.0)], &clip, 500);
        assert_eq!(value_at(&pasted, 600, false), Some(-10.0));
        assert_eq!(value_at(&pasted, 550, false), Some(-5.0));
        assert_eq!(delete_range(&pasted, 400, 700).len(), 2);
    }

    /// req: AUT-3
    #[test]
    fn moving_an_automated_control_offsets_the_whole_curve() {
        let mut p = Project::new("a");
        let mut t = Track::new_audio("t1".into(), "V", None, 1, 1);
        t.automation.push(AutomationLane {
            param: "fader".into(),
            points: vec![pt(0, -6.0), pt(48000, 0.0)],
            author: Author::Ai,
            visible: true,
        });
        p.tracks.push(t);
        let Command::SetTrack { track } = offset_lane(&p, "t1", "fader", -2.0).unwrap() else { panic!() };
        let l = track.lane("fader").unwrap();
        assert_eq!(l.points, vec![pt(0, -8.0), pt(48000, -2.0)]);
        assert_eq!(l.author, Author::User);
        // Values are clamped to the parameter range.
        let Command::SetTrack { track } = offset_lane(&p, "t1", "fader", 20.0).unwrap() else { panic!() };
        assert_eq!(track.lane("fader").unwrap().points[1].value, 12.0);
    }

    /// req: AUT-5
    #[test]
    fn section_offsets_become_points_with_ramps_at_the_boundaries() {
        let s = |id: &str, a, b| Section { id: id.into(), name: id.into(), color: 1, start: a, end: b };
        let sections = [s("s1", 0, 48000), s("s2", 48000, 96000), s("s3", 96000, 144000)];
        let pts = section_points(&sections, -3.0, &[0.0, 1.5, 0.0], 3840);
        assert_eq!(value_at(&pts, 10_000, false), Some(-3.0));
        assert_eq!(value_at(&pts, 70_000, false), Some(-1.5));
        assert_eq!(value_at(&pts, 120_000, false), Some(-3.0));
        // Ramp centred on the boundary: half way at the boundary itself.
        assert!((value_at(&pts, 48000, false).unwrap() - (-2.25)).abs() < 1e-9);
        assert_eq!(value_at(&pts, 48000 - 1920, false), Some(-3.0));
        assert!(section_points(&sections, 0.0, &[0.0, 0.0, 0.0], 3840).is_empty());
    }
}
