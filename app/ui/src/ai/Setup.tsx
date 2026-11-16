// Assistant setup (design 14a–14h; AI-24, AI-11): connecting an OpenRouter account from the browser, credit, and the
// manual path for a key the user already has. One dialog, opened from the first launch (1b), Settings › AI (11b) and
// the assistant column (13e, 14i–14k).
import { useEffect, useRef, useState, type ReactNode } from "react";
import type { ConnectOutcome } from "../backend/types";
import { Icon } from "../shared/Icon";
import { Switch } from "../settings/Settings";
import { useApp, useBackend } from "../state/context";
import { keyHome, usd } from "./Assist";

type Step = "connect" | "waiting" | "failed" | "manual" | "credit" | "ready";
const STEPS = ["Connect OpenRouter", "Add credit", "Ready"];
const AT: Record<Step, number> = { connect: 0, waiting: 0, failed: 0, manual: 0, credit: 1, ready: 3 };

const FAILURES: Partial<Record<ConnectOutcome, [string, string]>> = {
  denied: ["The connection was not approved", "Nothing was saved. You can try again, or paste a key you created yourself."],
  expired: ["The approval took longer than 10 minutes", "The page in the browser is no longer valid. Nothing was saved."],
  offline: ["OpenRouter could not be reached", "Check your connection. Nothing was saved."],
  refused: ["OpenRouter did not accept the connection", "Nothing was saved. You can try again, or paste a key you created yourself."],
  local: ["The browser could not be opened or answered", "Nothing was saved. You can paste a key you created yourself instead."],
};

export function Privacy() {
  return (
    <div className="tile row" style={{ height: "auto" }}>
      <Icon name="lock" className="aic" />
      <div className="col">
        <span className="b">Your audio never leaves this Mac</span>
        <span className="dim s1">The model receives measurements, roles, track names, your brief and your requests. Never audio, file names, paths or the song title.</span>
      </div>
    </div>
  );
}

function Rail({ step, failed }: { step: Step; failed: boolean }) {
  const at = AT[step];
  return (
    <ol className="steps setup-rail" aria-label="Setup steps">
      {STEPS.map((s, i) => {
        const state = i < at ? "done" : i === at ? (failed ? "fail" : "now") : "";
        return (
          <li key={s} className={`step ${state}`} aria-current={i === at ? "step" : undefined}>
            <span className="n">{state === "done" ? <Icon name="check" size="sm" /> : state === "fail" ? <Icon name="x" size="sm" /> : i + 1}</span>
            <span className="grow">{s}</span>
          </li>
        );
      })}
    </ol>
  );
}

function Numbered({ n, title, text, action }: { n: number; title: string; text: string; action?: ReactNode }) {
  return (
    <div className="tile row" style={{ height: "auto", alignItems: "flex-start" }}>
      <span className="badge">{n}</span>
      <div className="col g1 grow">
        <span className="b">{title}</span>
        <span className="dim s1">{text}</span>
      </div>
      {action}
    </div>
  );
}

export function Setup() {
  const backend = useBackend();
  const ai = useApp((s) => s.ai);
  const project = useApp((s) => s.project);
  const from = useApp((s) => s.setup);
  const setView = useApp((s) => s.setView);
  const openSettings = useApp((s) => s.openSettings);
  const [step, setStep] = useState<Step>(from === "manual" ? "manual" : from === "credit" ? "credit" : "connect");
  const [failure, setFailure] = useState<ConnectOutcome>("denied");
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [capText, setCapText] = useState((project?.ai.spend_cap_usd ?? 2).toFixed(2));
  const ref = useRef<HTMLDivElement>(null);
  const waiting = useRef(false);
  waiting.current = step === "waiting";

  useEffect(
    () =>
      backend.onAiConnect((o) => {
        // A "cancelled" is the echo of our own Cancel, or of a connection replaced by a newer one.
        if (o === "cancelled" || !waiting.current) return;
        if (o === "connected") setStep("credit");
        else {
          setFailure(o);
          setStep("failed");
        }
      }),
    [backend],
  );
  // The step changed: focus goes to its main action, so the keyboard carries on from there (UI-7).
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("[data-main]")?.focus();
  }, [step]);
  // Leaving the dialog ends a connection that is still waiting: nothing keeps listening behind the user's back.
  useEffect(() => () => void (waiting.current && backend.aiConnectCancel().catch(() => {})), [backend]);
  if (!ai) return null;

  const close = () => setView({ setup: null });
  const connect = () => {
    setError(null);
    setStep("waiting");
    backend.aiConnectStart().catch((e) => {
      const code = String(e).replace(/^Error: /, "") as ConnectOutcome;
      setFailure(code in FAILURES ? code : "local");
      setStep("failed");
    });
  };
  const cancel = () => {
    void backend.aiConnectCancel().catch(() => {});
    setStep("connect");
  };
  const demo = () => {
    void backend.aiSetConfig({ ...ai.config, offline_demo: true }).catch((e) => setError(String(e)));
    close();
  };
  const saveKey = () =>
    void backend
      .aiSetKey(key.trim())
      .then(() => backend.aiSetConfig({ ...ai.config, offline_demo: false }))
      .then(() => {
        setKey("");
        setStep("credit");
      })
      .catch((e) => setError(String(e)));
  const page = (which: "home" | "keys" | "credits") => void backend.aiOpenPage(which).catch((e) => setError(String(e)));
  const home = keyHome(ai.key_place);
  const cap = project?.ai.spend_cap_usd ?? null;
  const mixes = ai.typical_mix_usd > 0 ? Math.floor(5 / ai.typical_mix_usd) : 0;

  const sub =
    step === "credit"
      ? "Your OpenRouter account is connected. The assistant needs some credit on it to work."
      : step === "ready"
        ? "Everything is in place. You can change any of this in Settings › AI."
        : step === "manual"
          ? "Use a key you create yourself on openrouter.ai. Three steps, each opens the right page."
          : "Stemwise asks a language model for mix and master proposals. It reaches the model through OpenRouter, with an account that is yours.";

  let body: ReactNode;
  let foot: ReactNode;
  if (step === "connect") {
    body = (
      <>
        <div className="col g2">
          <span className="b">What is OpenRouter?</span>
          <span className="dim">
            A service that gives one account access to many AI models. You pay OpenRouter for what the model uses, a few cents per proposal. Stemwise adds nothing on top and has no account system of its own.
          </span>
        </div>
        <div className="col g2">
          <button type="button" className="btn primary lg" data-main onClick={connect}>
            <Icon name="link" size="m" />
            Connect with OpenRouter
          </button>
          <span className="faint s1">
            Opens openrouter.ai in your browser. Sign in or create an account there, then approve Stemwise. A key made for this app comes back by itself and is kept in {home}.
          </span>
        </div>
        <Privacy />
      </>
    );
    foot = (
      <>
        <button type="button" className="btn quiet" onClick={() => setStep("manual")}>I already have a key</button>
        <button type="button" className="btn quiet" onClick={demo}>Use offline demo</button>
        <span className="grow" />
        <button type="button" className="btn" onClick={close}>Not now</button>
      </>
    );
  } else if (step === "waiting") {
    body = (
      <>
        <div className="col g2" role="status">
          <div className="fx between">
            <span className="b">Waiting for your approval in the browser</span>
            <span className="faint s1">valid for 10 minutes</span>
          </div>
          <div className="progress ai indet" role="progressbar" aria-label="Waiting for the browser">
            <i />
          </div>
          <span className="dim">
            Sign in or create your OpenRouter account in the page that just opened, then choose <span className="b">Authorize</span>. This window continues by itself.
          </span>
        </div>
        <div className="fx g2">
          <button type="button" className="btn" data-main onClick={() => void backend.aiConnectReopen().catch((e) => setError(String(e)))}>
            <Icon name="link" size="m" />
            Open the page again
          </button>
          <button type="button" className="btn quiet" onClick={cancel}>Cancel</button>
        </div>
        <Privacy />
      </>
    );
    foot = (
      <>
        <button type="button" className="btn quiet" onClick={() => { cancel(); setStep("manual"); }}>I already have a key</button>
        <span className="grow" />
        <button type="button" className="btn" onClick={close}>Not now</button>
      </>
    );
  } else if (step === "failed") {
    const [title, text] = FAILURES[failure] ?? FAILURES.local!;
    body = (
      <>
        <div className="banner err" role="alert" style={{ alignItems: "flex-start" }} data-testid="connect-failed">
          <Icon name="warn" />
          <div className="col g1 grow">
            <span className="b">{title}</span>
            <span className="dim s1">{text}</span>
          </div>
        </div>
        <div className="fx g2">
          <button type="button" className="btn primary" data-main onClick={connect}>
            <Icon name="link" size="m" />
            Try again
          </button>
          <button type="button" className="btn" onClick={() => setStep("manual")}>Paste a key instead</button>
        </div>
        <Privacy />
      </>
    );
    foot = (
      <>
        <button type="button" className="btn quiet" onClick={demo}>Use offline demo</button>
        <span className="grow" />
        <button type="button" className="btn" onClick={close}>Not now</button>
      </>
    );
  } else if (step === "manual") {
    body = (
      <>
        <Numbered n={1} title="Create an OpenRouter account" text="Free. Skip this if you already have one." action={<button type="button" className="btn sm" onClick={() => page("home")}><Icon name="link" size="m" />Open OpenRouter</button>} />
        <Numbered n={2} title="Create a key" text="In your account: Keys › Create key. Name it “Stemwise”; a credit limit on the key is a good idea." action={<button type="button" className="btn sm" onClick={() => page("keys")}><Icon name="link" size="m" />Open keys page</button>} />
        <Numbered n={3} title="Paste it here" text={`It starts with sk-or-. It goes straight to ${home}: never into the project, the logs or the settings file.`} />
        <div className="fx g2">
          <input
            className="input grow"
            type="password"
            data-main
            aria-label="OpenRouter key"
            placeholder="Paste your key"
            autoComplete="off"
            spellCheck={false}
            value={key}
            onChange={(e) => setKey(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && key.trim() && saveKey()}
          />
          <button type="button" className="btn primary" disabled={!key.trim()} onClick={saveKey}>Save key</button>
        </div>
      </>
    );
    foot = (
      <>
        <button type="button" className="btn quiet" onClick={() => setStep("connect")}>
          <Icon name="link" size="m" />
          Connect with OpenRouter instead
        </button>
        <span className="grow" />
        <button type="button" className="btn" onClick={close}>Not now</button>
      </>
    );
  } else if (step === "credit") {
    body = (
      <>
        <div className="col g2">
          <span className="b">Add credit on OpenRouter</span>
          <span className="dim">Credit is prepaid: you add an amount once and each request takes a little from it. Nothing is charged while you are not using the assistant.</span>
        </div>
        <div className="fx g2 stretch">
          <div className="stat sm"><span className="faint s1">One mix proposal</span><span className="v" data-testid="typical-mix">≈ {usd(ai.typical_mix_usd)}</span><span className="faint s1">12 tracks, up to 3 passes</span></div>
          <div className="stat sm"><span className="faint s1">One revision</span><span className="v">≈ {usd(ai.typical_revision_usd)}</span></div>
          <div className="stat sm"><span className="faint s1">$5 of credit</span><span className="v">≈ {mixes} mixes</span><span className="faint s1">with {ai.config.model.split("/").pop()}</span></div>
        </div>
        <div className="fx g2">
          <button type="button" className="btn primary" data-main onClick={() => page("credits")}>
            <Icon name="link" size="m" />
            Open the credits page
          </button>
          <button type="button" className="btn" onClick={() => setStep("ready")}>I have added credit</button>
        </div>
        <div className="tile row" style={{ height: "auto" }}>
          <Icon name="info" />
          <span className="dim s1">Stemwise cannot see your balance: OpenRouter does not share it with apps. If the credit runs out, the assistant says so at the next request and nothing in your mix changes.</span>
        </div>
        {project ? (
          <div className="fx g3">
            <Switch on={cap !== null} label="Ask me before this project spends more than" onChange={(on) => void backend.setSpendCap(on ? Number(capText) || 2 : null).catch((e) => setError(String(e)))} />
            <span className="field">
              <input
                className="bare num"
                aria-label="Spending cap in dollars"
                inputMode="decimal"
                value={capText}
                onChange={(e) => setCapText(e.target.value)}
                onBlur={() => {
                  const v = Number(capText.replace(",", "."));
                  if (!Number.isFinite(v) || v <= 0) setCapText((cap ?? 2).toFixed(2));
                  else if (cap !== null && v !== cap) void backend.setSpendCap(v).catch((e) => setError(String(e)));
                }}
              />
              <span className="u">$</span>
            </span>
          </div>
        ) : (
          <span className="faint s1">Each project can have a spending cap: the assistant asks before going over it. Set it in Settings › AI once a project is open.</span>
        )}
      </>
    );
    foot = (
      <>
        <span className="faint s1">Estimates at the price of the selected model; every request shows its own before it starts.</span>
        <span className="grow" />
        <button type="button" className="btn quiet" onClick={() => setStep("ready")}>Skip for now</button>
      </>
    );
  } else {
    body = (
      <>
        <div className="tile row" style={{ height: "auto" }} data-testid="setup-ready">
          <Icon name="check" className="okc" />
          <div className="col grow">
            <span className="b">Assistant ready</span>
            <span className="dim s1">Key kept in {home}</span>
          </div>
        </div>
        <div className="fx g2 stretch">
          <div className="stat sm"><span className="faint s1">Model</span><span className="b ell">{ai.config.model}</span><span className="faint s1">change in Settings</span></div>
          <div className="stat sm"><span className="faint s1">Spending cap</span><span className="b">{project ? (cap !== null ? `${usd(cap)} for this project` : "None for this project") : "Set per project"}</span><span className="faint s1">asks before going over</span></div>
        </div>
        <span className="dim s1">To disconnect, remove the key in Settings › AI, or delete the key named “Stemwise” in your OpenRouter account.</span>
        <Privacy />
      </>
    );
    foot = (
      <>
        <button type="button" className="btn quiet" onClick={() => { close(); openSettings("ai"); }}>Open Settings</button>
        <span className="grow" />
        <button type="button" className="btn primary" data-main onClick={close}>Done</button>
      </>
    );
  }

  return (
    <div className="scrim">
      <div
        className="dialog l"
        role="dialog"
        aria-modal="true"
        aria-label="Set up the assistant"
        ref={ref}
        data-step={step}
        onKeyDown={(e) => {
          if (e.key === "Escape") close();
          if (e.key === "Tab") {
            const f = [...(ref.current?.querySelectorAll<HTMLElement>("button:not(:disabled), input") ?? [])];
            const i = f.indexOf(document.activeElement as HTMLElement);
            e.preventDefault();
            f[(i + (e.shiftKey ? -1 : 1) + f.length) % f.length]?.focus();
          }
        }}
      >
        <div className="dlg-h">
          <h2 className="h l" style={{ margin: 0 }}>Set up the assistant</h2>
          <span className="dim text">{sub}</span>
        </div>
        <div className="dlg-b">
          <div className="fx g5" style={{ alignItems: "flex-start" }}>
            <Rail step={step} failed={step === "failed"} />
            <div className="col g4 grow">
              {body}
              {error && <span className="dangerc s1" role="alert">{error}</span>}
            </div>
          </div>
        </div>
        <div className="dlg-f">{foot}</div>
      </div>
    </div>
  );
}
