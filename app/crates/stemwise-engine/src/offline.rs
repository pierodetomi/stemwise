//! Offline render (ENG-4): the graph's `process`, fed synchronously from the clips, faster than real time and
//! deterministic. The output is aligned with the timeline: the graph latency (ENG-5) is removed.

use std::path::Path;

use stemwise_core::media::MediaError;
use stemwise_core::model::Project;

use crate::clips::TrackStream;
use crate::graph::{Feed, Graph};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Routing(#[from] stemwise_core::routing::RoutingError),
    #[error("the project has no sample rate yet (nothing imported)")]
    NoRate,
    #[error("cancelled")]
    Cancelled,
    /// UI-6: offline renders (bounce, evaluation, CLI) never treat a missing file as silence.
    #[error("{0} audio file(s) missing from the project folder: locate them first")]
    Missing(usize),
}

const CHUNK: usize = 8192;

/// Buffered sequential feed from the track streams.
pub struct StreamFeed {
    streams: Vec<TrackStream>,
    bufs: Vec<Vec<f32>>,
    pos: Vec<usize>,
}

impl StreamFeed {
    pub fn open(package: &Path, project: &Project, graph: &Graph, start: i64) -> Result<Self, MediaError> {
        let mut streams = Vec::new();
        for id in &graph.stream_tracks {
            let t = project.track(id).expect("graph built from this project");
            let mut s = TrackStream::open(package, project, t)?;
            s.seek(start);
            streams.push(s);
        }
        let n = streams.len();
        Ok(StreamFeed { streams, bufs: vec![Vec::new(); n], pos: vec![0; n] })
    }
}

impl Feed for StreamFeed {
    fn fill(&mut self, stream: usize, out: &mut [f32]) {
        let mut done = 0;
        while done < out.len() {
            if self.pos[stream] >= self.bufs[stream].len() {
                let ch = self.streams[stream].channels();
                self.bufs[stream].resize(CHUNK * ch, 0.0);
                if self.streams[stream].read(&mut self.bufs[stream]).is_err() {
                    self.bufs[stream].fill(0.0);
                }
                self.pos[stream] = 0;
            }
            let take = (out.len() - done).min(self.bufs[stream].len() - self.pos[stream]);
            out[done..done + take].copy_from_slice(&self.bufs[stream][self.pos[stream]..self.pos[stream] + take]);
            self.pos[stream] += take;
            done += take;
        }
    }
}

/// A rendered stereo mix.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl Rendered {
    pub fn interleaved(&self) -> Vec<f32> {
        self.left.iter().zip(&self.right).flat_map(|(l, r)| [*l, *r]).collect()
    }

    /// BLAKE3 of the samples (little-endian f32, interleaved): "same project ⇒ same hash" (P2, ENG-4).
    pub fn hash(&self) -> String {
        let mut h = blake3::Hasher::new();
        for (l, r) in self.left.iter().zip(&self.right) {
            h.update(&l.to_le_bytes());
            h.update(&r.to_le_bytes());
        }
        h.finalize().to_hex()[..32].to_string()
    }
}

/// Renders `[start, end)` of the timeline with blocks of `block` frames. `progress` gets the fraction done and
/// may return `false` to cancel.
pub fn render_range(
    package: &Path,
    project: &Project,
    start: i64,
    end: i64,
    block: usize,
    progress: &mut dyn FnMut(f64) -> bool,
) -> Result<Rendered, RenderError> {
    let rate = project.sample_rate.ok_or(RenderError::NoRate)?;
    let missing = stemwise_core::package::missing_media(package, project).len();
    if missing > 0 {
        return Err(RenderError::Missing(missing));
    }
    let mut g = Graph::build(project, f64::from(rate))?;
    // Offline only: independent nodes on several threads (same result, NFR-4).
    g.parallel = true;
    let lat = g.latency() as i64;
    let mut feed = StreamFeed::open(package, project, &g, start)?;
    g.reset(start);
    let frames = (end - start).max(0) as usize;
    let total = frames + lat as usize;
    let mut l = vec![0f32; total];
    let mut r = vec![0f32; total];
    let mut done = 0;
    while done < total {
        let n = block.min(total - done);
        g.process(start + done as i64, &mut l[done..done + n], &mut r[done..done + n], &mut feed);
        done += n;
        if !progress(done as f64 / total as f64) {
            return Err(RenderError::Cancelled);
        }
    }
    Ok(Rendered { rate, left: l.split_off(lat as usize), right: r.split_off(lat as usize) })
}

/// Renders the whole song: from 0 to the end of the last clip plus `tail_secs` (reverb and delay tails).
pub fn render_song(package: &Path, project: &Project, tail_secs: f64) -> Result<Rendered, RenderError> {
    let rate = project.sample_rate.ok_or(RenderError::NoRate)?;
    let end = project.content_end() + (tail_secs * f64::from(rate)) as i64;
    render_range(package, project, 0, end, 4096, &mut |_| true)
}
