// Screen 14 — Assistant setup (P-015): connecting an OpenRouter account, adding credit, the manual key path,
// and the "credit has run out" state of the assistant column.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { welcome } from './s-start.mjs';
const { ic, tag, btn, input, flabel, progress } = L;

const STEPS = ['Connect OpenRouter', 'Add credit', 'Ready'];
// states: one of 'done' | 'now' | 'fail' | '' per step
const rail = (states) => `<div class="steps" style="flex:none;width:calc(var(--dlg-s) / 2);padding-top:var(--sp-1)">${STEPS.map((s, i) => `<div class="step ${states[i]}"><span class="n">${states[i] === 'done' ? ic('check', 'sm') : states[i] === 'fail' ? ic('x', 'sm') : i + 1}</span><span class="grow">${s}</span></div>`).join('')}</div>`;
const body = (states, inner) => `<div class="fx g5" style="align-items:flex-start">${rail(states)}<div class="col g4 grow" style="min-width:0">${inner}</div></div>`;
const privacy = `<div class="tile row" style="height:auto">${ic('lock', 'aic')}<div class="col"><span class="b">Your audio never leaves this Mac</span><span class="dim s1">The model receives measurements, roles, track names, your brief and your requests. Never audio, file names, paths or the song title.</span></div></div>`;
const sw = (on, t) => `<span class="fx g2"><span class="switch ${on ? 'on' : ''}" role="switch" aria-checked="${on}"></span><span>${t}</span></span>`;
const num = (n, title, text, action = '') => `<div class="tile row" style="height:auto;align-items:flex-start"><span class="badge">${n}</span><div class="col g1 grow"><span class="b">${title}</span><span class="dim s1">${text}</span></div>${action}</div>`;
const TITLE = 'Set up the assistant';
const dlg = (o) => L.dialog({ size: 'l', title: TITLE, ...o });
const over = (d) => welcome(true).replace(/<\/div>$/, `${d}</div>`);

const connect = dlg({
  sub: 'Stemwise asks a language model for mix and master proposals. It reaches the model through OpenRouter, with an account that is yours.',
  body: body(['now', '', ''], `<div class="col g2"><span class="b">What is OpenRouter?</span><span class="dim">A service that gives one account access to many AI models. You pay OpenRouter for what the model uses, a few cents per proposal. Stemwise adds nothing on top and has no account system of its own.</span></div>
    <div class="col g2">${btn('Connect with OpenRouter', 'primary lg', 'link')}<span class="faint s1">Opens openrouter.ai in your browser. Sign in or create an account there, then approve Stemwise. A key made for this app comes back by itself and is kept in the macOS keychain.</span></div>
    ${privacy}`),
  foot: `${btn('I already have a key', 'quiet')}${btn('Use offline demo', 'quiet')}<span class="grow"></span>${btn('Not now')}`,
});

const waiting = dlg({
  sub: 'Stemwise asks a language model for mix and master proposals. It reaches the model through OpenRouter, with an account that is yours.',
  body: body(['now', '', ''], `<div class="col g2"><div class="fx between"><span class="b">Waiting for your approval in the browser</span><span class="faint s1">valid for 10 minutes</span></div>${progress(0, 'ai indet')}<span class="dim">Sign in or create your OpenRouter account in the page that just opened, then choose <span class="b">Authorize</span>. This window continues by itself.</span></div>
    <div class="fx g2">${btn('Open the page again', '', 'link')}${btn('Cancel', 'quiet')}</div>
    ${privacy}`),
  foot: `${btn('I already have a key', 'quiet')}<span class="grow"></span>${btn('Not now')}`,
});

const failed = (kind) => dlg({
  sub: 'Stemwise asks a language model for mix and master proposals. It reaches the model through OpenRouter, with an account that is yours.',
  body: body(['fail', '', ''], `<div class="banner err" style="align-items:flex-start">${ic('warn')}<div class="col g1 grow"><span class="b">${{ denied: 'The connection was not approved', expired: 'The approval took longer than 10 minutes', offline: 'OpenRouter could not be reached' }[kind]}</span><span class="dim s1">${{ denied: 'Nothing was saved. You can try again, or paste a key you created yourself.', expired: 'The page in the browser is no longer valid. Nothing was saved.', offline: 'Check your connection. Nothing was saved.' }[kind]}</span></div></div>
    <div class="fx g2">${btn('Try again', 'primary', 'link')}${btn('Paste a key instead')}</div>
    ${privacy}`),
  foot: `${btn('Use offline demo', 'quiet')}<span class="grow"></span>${btn('Not now')}`,
});

const credit = dlg({
  sub: 'Your OpenRouter account is connected. The assistant needs some credit on it to work.',
  body: body(['done', 'now', ''], `<div class="col g2"><span class="b">Add credit on OpenRouter</span><span class="dim">Credit is prepaid: you add an amount once and each request takes a little from it. Nothing is charged while you are not using the assistant.</span></div>
    <div class="fx g2 stretch"><div class="stat sm"><span class="faint s1">One mix proposal</span><span class="v">≈ $0.10</span><span class="faint s1">12 tracks, 2 passes</span></div><div class="stat sm"><span class="faint s1">One revision</span><span class="v">≈ $0.04</span></div><div class="stat sm"><span class="faint s1">$5 of credit</span><span class="v">≈ 40 mixes</span><span class="faint s1">with the default model</span></div></div>
    <div class="fx g2">${btn('Open the credits page', 'primary', 'link')}${btn('I have added credit')}</div>
    <div class="tile row" style="height:auto">${ic('info')}<span class="dim s1">Stemwise cannot see your balance: OpenRouter does not share it with apps. If the credit runs out, the assistant says so at the next request and nothing in your mix changes.</span></div>
    <div class="fx g3">${sw(true, 'Ask me before a project spends more than')}${L.field('2.00', '$')}</div>`),
  foot: `<span class="faint s1">Costs are estimates from this app’s own history; every request shows its estimate before it starts.</span><span class="grow"></span>${btn('Skip for now', 'quiet')}`,
});

const ready = dlg({
  sub: 'Everything is in place. You can change any of this in Settings › AI.',
  body: body(['done', 'done', 'done'], `<div class="tile row" style="height:auto">${ic('check', 'okc')}<div class="col grow"><span class="b">Assistant ready</span><span class="dim s1">Connected with OpenRouter · key “Stemwise” kept in the macOS keychain</span></div></div>
    <div class="fx g2 stretch"><div class="stat sm"><span class="faint s1">Model</span><span class="b ell">openai/gpt-6.1-sol</span><span class="faint s1">default · change in Settings</span></div><div class="stat sm"><span class="faint s1">Spending cap</span><span class="b">$2.00 per project</span><span class="faint s1">asks before going over</span></div></div>
    <span class="dim s1">To disconnect, remove the key in Settings › AI, or delete the key named “Stemwise” in your OpenRouter account.</span>
    ${privacy}`),
  foot: `${btn('Open Settings', 'quiet')}<span class="grow"></span>${btn('Done', 'primary')}`,
});

const manual = (filled) => dlg({
  sub: 'Use a key you create yourself on openrouter.ai. Three steps, each opens the right page.',
  body: body(['now', '', ''], `${num(1, 'Create an OpenRouter account', 'Free, with an e-mail address or a Google or GitHub account.', btn('Open sign-up', 'sm', 'link'))}
    ${num(2, 'Create a key', 'In your account: Keys › Create key. Name it “Stemwise”; a credit limit on the key is a good idea.', btn('Open keys page', 'sm', 'link'))}
    ${num(3, 'Paste it here', 'It starts with sk-or-. It goes straight to the macOS keychain: never into the project, the logs or the settings file.', '')}
    <div class="fx g2">${input(filled ? 'sk-or-••••••••••••••••••••••••••••3f9a' : 'Paste your key', 'grow', !filled)}${btn('Save key', 'primary ' + (filled ? '' : 'is-dis'))}</div>`),
  foot: `${btn('Connect with OpenRouter instead', 'quiet', 'link')}<span class="grow"></span>${btn('Not now')}`,
});

// The page the browser lands on after approval. Served by the app on this Mac only, for one request.
const landing = (ok) => `<div class="dialog s" role="document" aria-label="Browser page"><div class="dlg-h"><div class="fx g2">${L.mark({ brand: true })}<span class="h l">Stemwise</span></div></div><div class="dlg-b"><div class="tile row" style="height:auto">${ic(ok ? 'check' : 'x', ok ? 'okc' : 'dangerc')}<div class="col"><span class="b">${ok ? 'Connected' : 'Not connected'}</span><span class="dim s1">${ok ? 'You can close this tab and go back to Stemwise.' : 'The request was declined or has expired. Go back to Stemwise to try again.'}</span></div></div><span class="faint s1">This page comes from the Stemwise app on your Mac, not from the internet.</span></div></div>`;
const landings = `<div class="sw free" data-phase="mix"><div class="sheet c2" style="grid-template-columns:repeat(2,max-content)">${landing(true)}${landing(false)}</div></div>`;

const asstState = (kind) => {
  const m = { credit: ['warn', 'Your OpenRouter credit has run out', 'The request was not sent and nothing in your mix changed. Add credit, then ask again.', btn('Add credit…', 'primary', 'link') + btn('Use offline demo')],
    cap: ['warn', 'The key has reached its own limit', 'You set a credit limit on the key “Stemwise” in your OpenRouter account. Raise it there, or connect again for a new key.', btn('Open keys page', 'primary', 'link') + btn('Connect again')],
    revoked: ['key', 'OpenRouter no longer accepts this key', 'It was deleted or has expired. Connect again to get a new one.', btn('Connect with OpenRouter', 'primary', 'link') + btn('Use offline demo')] }[kind];
  return P.assistant({ asstBody: `${P.brief()}<div class="sect"><div class="banner ${kind === 'revoked' ? 'err' : ''}" style="align-items:flex-start">${ic(m[0])}<div class="col g2 grow"><span class="b">${m[1]}</span><span class="dim s1">${m[2]}</span><div class="fx g2 wrap">${m[3]}</div></div></div></div>${P.askFoot({ ctaCls: 'is-dis' })}` });
};

export default {
  'screens/14-assistant-setup.html': P.page('14 · Assistant setup', 'Connecting the assistant without copying a key: the app opens OpenRouter in the browser and receives a key made for it. Then credit, and the manual path for who already has a key. Opened from the first launch (1b), from Settings › AI (11b) and from the assistant column when no key is set (13e).', [
    P.frame('14a', 'Connect', 'Step 1 of 3. One button; what OpenRouter is in two lines; the privacy statement. The manual path and the offline demo stay one click away.', over(connect)),
    P.frame('14b', 'Waiting for the browser', 'The browser is open on openrouter.ai. The dialog continues by itself when the user approves; the page can be reopened; the request expires after 10 minutes.', over(waiting)),
    P.frame('14c', 'Page shown in the browser', 'What the browser lands on after approving or declining. It is served by the app on this Mac for that one request.', landings),
    P.frame('14d', 'Add credit', 'Step 2. What a proposal costs, a button to the credits page, and the honest limit: the app cannot read the balance. The spending cap is set here too.', over(credit)),
    P.frame('14e', 'Ready', 'Step 3. Where the key is, which model, the cap, and how to disconnect.', over(ready)),
    P.frame('14f', 'Not approved', 'The user declined in the browser. Same shape for the two other failures: “The approval took longer than 10 minutes” and “OpenRouter could not be reached”.', over(failed('denied'))),
    P.frame('14g', 'I already have a key', 'The manual path: three numbered steps, each with the link to the right page, then the field. Saving leads to 14d.', over(manual(false))),
    P.frame('14h', 'Key pasted', 'The key is masked once typed. “Save key” is enabled.', over(manual(true))),
    P.frame('14i', 'Credit has run out', 'The provider refused a request for lack of credit. The assistant column says so and links to the credits page; the request button is disabled until then.', P.mixScreen({ asstPanel: asstState('credit') })),
    P.frame('14j', 'Key limit reached', 'The key carries a limit set by the user on OpenRouter and has used it up.', P.mixScreen({ asstPanel: asstState('cap') })),
    P.frame('14k', 'Key no longer valid', 'The key was deleted on OpenRouter or has expired.', P.mixScreen({ asstPanel: asstState('revoked') })),
  ]),
};
