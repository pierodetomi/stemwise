"""Mock decider (DEC-7, D-043): simple, deterministic rules on roles and analysis, no network.

It is a test and development tool, not a mixer: the numbers are plausible, not tuned to sound
good. Rationales are fixed English templates (it does not follow the language of the brief).
Rules:

- gain by role (`ROLE_GAIN_DB`); when the drum family (`DRUM_ROLES`) is split over n > 1 stems,
  each gets a further -10·log10(n) dB, so the kit sums to about the loudness of one stem;
- pan: stereo stems and mono kick, snare, bass and lead vocal at the centre; the other mono
  stems alternate -0.5 / +0.5 within their role, in project order (a lone one stays centred);
- high-pass by role (`ROLE_HPF_HZ`), no low-pass;
- EQ: for each collision of `masking_top` with score >= `MIN_COLLISION_SCORE` that involves a
  priority stem (lead vocal > bass > kick), a -2 dB peak (Q 1) at the geometric centre of the
  band on the other stem (the lower-priority one when both are priority stems), at most
  `MAX_EQ_CUTS` per stem and only below 0.9 of the Nyquist frequency;
- a fixed compressor on lead vocal and bass;
- a fixed shared reverb, with sends by role (`ROLE_SEND_DB`); no sends ⇒ no reverb.

From iteration 1 (D-057) it starts from the previous plan and makes one correction towards the
target: in the band with the largest `|band_delta_db|` of the last evaluation (if at least
`FEEDBACK_MIN_DB`), the stem with the largest `band_share_pct` in that band gets a `peak` (Q 1,
geometric centre of the band) of `band_delta_db` limited to ±`FEEDBACK_MAX_DB`: the peak already
at that frequency is adjusted, otherwise one is added if the stem has fewer than 4 bands. Ties go
to the lower band and to project order. Without a target there is no correction.

With an author's request (a revision, D-065) it starts from the previous plan (or from the rules
above when there is none) and applies keyword rules, clause by clause: the request is lower-cased
and split at `,` `;` `.` and the words `e`/`and`. In a clause, the subjects are the stem ids it
names and the stems of the roles it names (`ROLE_WORDS`, whole words; `backing vocals` counts as
backing vocals only), the direction is up or down (`UP_WORDS`, `DOWN_WORDS`, whole words or
phrases). Without a reverb word the subjects' `gain_db` moves by ±`REQUEST_GAIN_DB`; with one
(`REVERB_WORDS`) their `reverb_send_db` moves by ±`REQUEST_SEND_DB` (all stems with a send when
no stem is named; a stem without a send gets `NEW_SEND_DB` on "up"). Values stay within the plan
limits. A clause without a subject, without a direction or with both directions changes nothing
(so "voce e basso più avanti" moves only the bass: `e` splits the clause); if no clause changes
anything the summary says the request was not understood, or that the parameters asked for are
already at their limit.
"""

from __future__ import annotations

import math
import re
from collections import Counter, defaultdict

from stemwise.analysis.spectral import BAND_LABELS, BANDS_HZ
from stemwise.decision.base import Decision, DecisionInput
from stemwise.decision.prompt import build_messages
from stemwise.decision.schema import (
    MAX_EQ_BANDS,
    PLAN_VERSION,
    Compressor,
    EqBand,
    MixPlan,
    Reverb,
    StemPlan,
)

ROLE_GAIN_DB: dict[str, float] = {
    "kick": 0.0,
    "snare": 0.0,
    "toms": -2.0,
    "overheads": -2.0,
    "drums": 0.0,
    "percussion": -4.0,
    "bass": 0.0,
    "electric_guitar": -2.0,
    "acoustic_guitar": -2.0,
    "keys": -2.0,
    "synth": -2.0,
    "pad": -6.0,
    "lead_vocal": 2.0,
    "backing_vocal": -4.0,
    "fx": -6.0,
    "other": -3.0,
}
DRUM_ROLES = frozenset({"kick", "snare", "toms", "overheads", "drums", "percussion"})
CENTRED_ROLES = frozenset({"kick", "snare", "bass", "lead_vocal"})
SIDE_PAN = 0.5
ROLE_HPF_HZ: dict[str, float | None] = {
    "kick": None,
    "snare": 80.0,
    "toms": 60.0,
    "overheads": 120.0,
    "drums": None,
    "percussion": 120.0,
    "bass": None,
    "electric_guitar": 90.0,
    "acoustic_guitar": 90.0,
    "keys": 80.0,
    "synth": 60.0,
    "pad": 120.0,
    "lead_vocal": 90.0,
    "backing_vocal": 120.0,
    "fx": 120.0,
    "other": 80.0,
}
PRIORITY_ROLES = ("lead_vocal", "bass", "kick")  # highest first
MIN_COLLISION_SCORE = 0.05
EQ_CUT_DB = -2.0
EQ_CUT_Q = 1.0
MAX_EQ_CUTS = 2
COMPRESSORS: dict[str, Compressor] = {
    "lead_vocal": Compressor(
        threshold_db=-24.0, ratio=3.0, attack_ms=10.0, release_ms=120.0, makeup_db=2.0
    ),
    "bass": Compressor(
        threshold_db=-24.0, ratio=3.0, attack_ms=20.0, release_ms=200.0, makeup_db=2.0
    ),
}
REVERB = Reverb(room_size=0.3, damping=0.5, return_hpf_hz=250.0, return_gain_db=-6.0)
ROLE_SEND_DB: dict[str, float] = {
    "lead_vocal": -20.0,
    "backing_vocal": -18.0,
    "snare": -26.0,
    "electric_guitar": -24.0,
    "acoustic_guitar": -24.0,
    "keys": -24.0,
    "pad": -22.0,
}


FEEDBACK_MIN_DB = 1.0
FEEDBACK_MAX_DB = 3.0  # the EQ change limit of DEC-5
MAX_EQ_GAIN_DB = 9.0


def _band_centre_hz(band: int) -> float:
    lo, hi = BANDS_HZ[band]
    return float(round(math.sqrt(lo * hi)))


def _feedback(inp: DecisionInput, previous: MixPlan) -> tuple[MixPlan, str]:
    """D-057: one EQ correction of the previous plan towards the target."""
    evaluation = inp.history[-1].evaluation if inp.history else None
    deltas = (evaluation or {}).get("band_delta_db")
    unchanged = previous.model_copy(update={"iteration": inp.iteration})
    if not deltas:
        return unchanged, "no target to correct towards"
    band = max(range(len(deltas)), key=lambda i: (abs(deltas[i]), -i))
    delta = deltas[band]
    label = BAND_LABELS[band]
    if abs(delta) < FEEDBACK_MIN_DB:
        return unchanged, f"largest band difference below {FEEDBACK_MIN_DB:g} dB"
    freq = _band_centre_hz(band)
    if freq >= 0.9 * inp.sample_rate / 2.0:
        return unchanged, f"band {label} too close to the Nyquist frequency"
    shares = inp.analysis.band_share_pct
    order = {s.id: i for i, s in enumerate(inp.stems)}
    sid = max(order, key=lambda s: (shares[s][band], -order[s]))
    correction = max(-FEEDBACK_MAX_DB, min(FEEDBACK_MAX_DB, delta))
    sp = previous.stems[sid]
    eq = list(sp.eq_bands)
    at = next((i for i, b in enumerate(eq) if b.type == "peak" and b.freq_hz == freq), None)
    if at is not None:
        gain = max(-MAX_EQ_GAIN_DB, min(MAX_EQ_GAIN_DB, round(eq[at].gain_db + correction, 1)))
        eq[at] = eq[at].model_copy(update={"gain_db": gain + 0.0})
    elif len(eq) < MAX_EQ_BANDS:
        gain = round(correction, 1) + 0.0
        eq.append(EqBand(type="peak", freq_hz=freq, gain_db=gain, q=EQ_CUT_Q))
    else:
        return unchanged, f"{sid} has no free EQ band"
    note = (
        f" Iteration {inp.iteration}: peak at {freq:g} Hz now {gain:+.1f} dB, towards the "
        f"target in the {label} band (difference {delta:+.1f} dB)."
    )
    stems = dict(previous.stems)
    stems[sid] = sp.model_copy(update={"eq": eq, "rationale": sp.rationale + note})
    plan = previous.model_copy(update={"iteration": inp.iteration, "stems": stems})
    return plan, f"{correction:+.1f} dB at {freq:g} Hz on {sid} ({label} band)"


REQUEST_GAIN_DB = 2.0
REQUEST_SEND_DB = 3.0
NEW_SEND_DB = -24.0
MIN_GAIN_DB, MAX_GAIN_DB = -24.0, 12.0
MIN_SEND_DB, MAX_SEND_DB = -60.0, -6.0
ROLE_WORDS: dict[str, tuple[str, ...]] = {
    "lead_vocal": ("voce", "vocal", "vocals", "voice", "cantato"),
    "backing_vocal": ("cori", "coro", "backing"),
    "bass": ("basso", "bass"),
    "kick": ("cassa", "kick"),
    "snare": ("rullante", "snare"),
    "drums": ("batteria", "drums", "drum"),  # the whole drum family (DRUM_ROLES)
    "electric_guitar": ("chitarra", "chitarre", "guitar", "guitars"),
    "acoustic_guitar": ("chitarra", "chitarre", "guitar", "guitars"),
    "keys": ("tastiera", "tastiere", "keys"),
    "pad": ("pad", "pads"),
    "synth": ("synth", "synths"),
}
REVERB_WORDS = ("riverbero", "riverberi", "reverb")
UP_WORDS = (
    "più avanti", "avanti", "più forte", "più presente", "alza", "alzare", "aumenta",
    "louder", "forward", "upfront", "up", "more",
)  # fmt: skip
DOWN_WORDS = (
    "più indietro", "indietro", "più piano", "più bassa", "più basse", "meno", "abbassa",
    "abbassare", "riduci", "quieter", "softer", "lower", "back", "down", "less",
)  # fmt: skip
_CLAUSES = re.compile(r"[,;.]|\b(?:e|and)\b")


def _has(text: str, phrase: str) -> bool:
    return re.search(rf"(?<![\w]){re.escape(phrase)}(?![\w])", text) is not None


def _subjects(clause: str, inp: DecisionInput) -> list[str]:
    roles = {s.id: s.role for s in inp.stems}
    named = {sid for sid in roles if _has(clause, sid)}
    words = re.sub(r"\bbacking vocals?\b", "backing", clause)
    for role, synonyms in ROLE_WORDS.items():
        if any(_has(words, w) for w in synonyms):
            family = DRUM_ROLES if role == "drums" else {role}
            named |= {sid for sid, r in roles.items() if r in family}
    return [s.id for s in inp.stems if s.id in named]  # project order


def _direction(clause: str) -> int:
    up = any(_has(clause, w) for w in UP_WORDS)
    down = any(_has(clause, w) for w in DOWN_WORDS)
    return 0 if up == down else (1 if up else -1)


def _limited(value: float, lo: float, hi: float) -> float:
    return round(min(hi, max(lo, value)), 1) + 0.0


def _apply_request(inp: DecisionInput, start: MixPlan) -> tuple[MixPlan, list[str], list[str]]:
    """D-065: keyword rules of the author's request on the starting plan."""
    stems = dict(start.stems)
    changes: list[str] = []
    saturated: list[str] = []
    text = " ".join((inp.request or "").lower().split())
    for clause in (c.strip() for c in _CLAUSES.split(text)):
        if not clause:
            continue
        direction = _direction(clause)
        reverb = any(_has(clause, w) for w in REVERB_WORDS)
        subjects = _subjects(clause, inp)
        if reverb and not subjects:
            subjects = [sid for sid, sp in stems.items() if sp.reverb_send_db is not None]
        if direction == 0 or not subjects:
            continue
        for sid in subjects:
            sp = stems[sid]
            if reverb:
                if sp.reverb_send_db is None:
                    if direction < 0:
                        continue
                    value = NEW_SEND_DB
                else:
                    value = _limited(
                        sp.reverb_send_db + direction * REQUEST_SEND_DB, MIN_SEND_DB, MAX_SEND_DB
                    )
                field, old = "reverb_send_db", sp.reverb_send_db
            else:
                value = _limited(sp.gain_db + direction * REQUEST_GAIN_DB, MIN_GAIN_DB, MAX_GAIN_DB)
                field, old = "gain_db", sp.gain_db
            what = "reverb send" if reverb else "gain"
            if value == old:
                saturated.append(f"{sid} {what}")
                continue
            note = f" Revision: {what} {value:+.1f} dB as requested ({clause!r})."
            stems[sid] = sp.model_copy(update={field: value, "rationale": sp.rationale + note})
            changes.append(f"{sid} {what} {value:+.1f} dB")
    return start.model_copy(update={"iteration": inp.iteration, "stems": stems}), changes, saturated


def _gains(inp: DecisionInput) -> tuple[dict[str, float], int]:
    n_drums = sum(1 for s in inp.stems if s.role in DRUM_ROLES)
    offset = -10.0 * math.log10(n_drums) if n_drums > 1 else 0.0
    gains = {
        s.id: round(ROLE_GAIN_DB[s.role] + (offset if s.role in DRUM_ROLES else 0.0), 1) + 0.0
        for s in inp.stems
    }
    return gains, n_drums


def _pans(inp: DecisionInput) -> dict[str, float]:
    channels = {sid: e.channels for sid, e in inp.analysis.stems.items()}
    side = [s for s in inp.stems if channels[s.id] == 1 and s.role not in CENTRED_ROLES]
    per_role = Counter(s.role for s in side)
    seen: dict[str, int] = defaultdict(int)
    pans = {s.id: 0.0 for s in inp.stems}
    for s in side:
        if per_role[s.role] > 1:
            pans[s.id] = -SIDE_PAN if seen[s.role] % 2 == 0 else SIDE_PAN
            seen[s.role] += 1
    return pans


def _cuts(inp: DecisionInput) -> dict[str, list[tuple[EqBand, str]]]:
    """EQ cuts from the strongest collisions, with the reason for the rationale."""
    roles = {s.id: s.role for s in inp.stems}
    rank = {role: i for i, role in enumerate(PRIORITY_ROLES)}
    limit_hz = 0.9 * inp.sample_rate / 2.0
    cuts: dict[str, list[tuple[EqBand, str]]] = defaultdict(list)
    for c in inp.analysis.masking_top:
        if c.score < MIN_COLLISION_SCORE:
            continue
        ra, rb = rank.get(roles[c.a]), rank.get(roles[c.b])
        if ra is None and rb is None:
            continue
        if rb is None or (ra is not None and ra < rb):
            keep, cut = c.a, c.b
        else:
            keep, cut = c.b, c.a
        lo, hi = BANDS_HZ[BAND_LABELS.index(c.band)]
        freq = float(round(math.sqrt(lo * hi)))
        if freq >= limit_hz or len(cuts[cut]) >= MAX_EQ_CUTS:
            continue
        if any(band.freq_hz == freq for band, _ in cuts[cut]):
            continue
        band = EqBand(type="peak", freq_hz=freq, gain_db=EQ_CUT_DB, q=EQ_CUT_Q)
        cuts[cut].append((band, f"{keep} ({c.band} collision, score {c.score:.2f})"))
    return cuts


def _rationale(role: str, sp: StemPlan, reasons: list[str], drum_offset: bool) -> str:
    parts = [f"Role {role}: gain {sp.gain_db:+.1f} dB"]
    if drum_offset:
        parts[0] += " (drum kit split over several stems)"
    parts.append("centred" if sp.pan == 0.0 else f"panned {sp.pan:+.1f}")
    if sp.hpf_hz is not None:
        parts.append(f"high-pass at {sp.hpf_hz:g} Hz")
    if reasons:
        parts.append("2 dB cut to make room for " + " and ".join(reasons))
    if sp.compressor is not None:
        parts.append("gentle compression for a steadier level")
    if sp.reverb_send_db is not None:
        parts.append(f"reverb send {sp.reverb_send_db:g} dB")
    return "; ".join(parts) + "."


class MockDecider:
    """Deterministic rule-based decider for tests and offline development (DEC-7)."""

    name = "mock"

    def decide(self, decision_input: DecisionInput) -> Decision:
        inp = decision_input
        # D-040: log the messages a real decider would send, so the prompt can be read offline.
        request_log = {"decider": self.name, "sent": False, "messages": build_messages(inp)}
        if inp.request:
            start = inp.previous_plan or self._rule_plan(inp)
            plan, changes, saturated = _apply_request(inp, start)
            if changes:
                summary = "Mock revision: " + "; ".join(changes) + "."
            elif saturated:
                summary = (
                    "Mock revision: " + ", ".join(saturated) + " already at the limit, "
                    "starting plan unchanged."
                )
            else:
                summary = "Mock revision: request not understood, starting plan unchanged."
            return Decision(
                plan=plan.model_copy(update={"summary": summary}), request_log=request_log
            )
        if inp.iteration >= 1 and inp.previous_plan is not None:
            plan, change = _feedback(inp, inp.previous_plan)
            summary = f"Mock plan, iteration {inp.iteration}: previous plan, {change}."
            plan = plan.model_copy(update={"summary": summary})
            return Decision(plan=plan, request_log=request_log)
        return Decision(plan=self._rule_plan(inp), request_log=request_log)

    @staticmethod
    def _rule_plan(inp: DecisionInput) -> MixPlan:
        """The plan of iteration 0: rules on roles and analysis (D-043)."""
        gains, n_drums = _gains(inp)
        pans = _pans(inp)
        cuts = _cuts(inp)
        stems: dict[str, StemPlan] = {}
        for s in inp.stems:
            draft = StemPlan(
                gain_db=gains[s.id],
                pan=pans[s.id],
                hpf_hz=ROLE_HPF_HZ[s.role],
                lpf_hz=None,
                eq=[band for band, _ in cuts.get(s.id, [])],
                compressor=COMPRESSORS.get(s.role),
                reverb_send_db=ROLE_SEND_DB.get(s.role),
                rationale="-",
            )
            reasons = [reason for _, reason in cuts.get(s.id, [])]
            text = _rationale(s.role, draft, reasons, s.role in DRUM_ROLES and n_drums > 1)
            stems[s.id] = draft.model_copy(update={"rationale": text})
        has_sends = any(sp.reverb_send_db is not None for sp in stems.values())
        n_cuts = sum(len(v) for v in cuts.values())
        reverb_text = "shared reverb on vocals and melodic parts" if has_sends else "no reverb"
        summary = (
            f"Mock plan: role-based gains, pans and filters; {n_cuts} EQ cut(s) from the masking "
            f"analysis; {reverb_text}."
        )
        return MixPlan(
            version=PLAN_VERSION,
            iteration=inp.iteration,
            summary=summary,
            reverb=REVERB if has_sends else None,
            stems=stems,
        )
