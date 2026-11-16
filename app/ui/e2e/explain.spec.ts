import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

// Explain this mix (F18, P-019) on the mock backend: the view (UI-14), the map of the signal (UI-15), the mix in
// words (UI-16). The mock stands in for the measured levels of ANA-20 with numbers derived from the document.

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}
const explainBtn = (page: Page) => page.getByRole("button", { name: "Explain", exact: true });
async function openExplain(page: Page) {
  await openDemo(page);
  await explainBtn(page).click();
  await expect(page.getByLabel("Mix map")).toBeVisible();
}
const node = (page: Page, name: string) => page.getByTestId(`node-${name}`);
const text = (page: Page) => page.getByTestId("explain-text");
const mock = <T>(page: Page, f: string) => page.evaluate(`window.__stemwiseMock.${f}`) as Promise<T>;
const ids = async (page: Page) => {
  const p = await mock<{ tracks: { id: string; name: string }[] }>(page, "getProject()");
  return (name: string) => p.tracks.find((t) => t.name === name)!.id;
};
const steps = async (page: Page) => (await mock<[unknown[], number]>(page, 'history("mix")'))[0].length;

// req: UI-14
test("Explain takes the place of the workspace, leaves the top bar live, changes nothing and closes with Esc", async ({ page }) => {
  await openDemo(page);
  const before = await steps(page);
  const saved = await mock<object>(page, "getProject()");
  await expect(explainBtn(page)).toHaveAttribute("aria-pressed", "false");
  await explainBtn(page).click();
  await expect(explainBtn(page)).toHaveAttribute("aria-pressed", "true");
  for (const region of ["Mix map", "Explanation", "Top bar"]) await expect(page.getByLabel(region, { exact: true })).toBeVisible();
  for (const region of ["Timeline", "Mixer", "Inspector", "Assistant"]) await expect(page.getByLabel(region, { exact: true })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Explain this mix" })).toBeVisible();
  await expect(page.getByText("Read from the project as it is now · no AI call, no cost")).toBeVisible();
  // The song plays while reading: transport and position are the workspace's own.
  await page.getByRole("button", { name: "Play" }).click();
  await expect(page.getByRole("button", { name: "Pause" })).toBeVisible();
  await expect(page.getByTestId("position")).not.toHaveText("0:00.00");
  await page.getByRole("button", { name: "Pause" }).click();
  // Looking around is not editing: no history step, the same document.
  await node(page, "Lead Vox").click();
  await page.getByRole("button", { name: "Chorus", exact: true }).first().click();
  await page.getByRole("button", { name: "Fold Drums" }).click();
  expect(await steps(page)).toBe(before);
  expect(await mock<object>(page, "getProject()")).toEqual(saved);
  // Esc: back to the whole mix, then out; ⌘E opens and closes it too.
  await page.keyboard.press("Escape");
  await expect(page.getByRole("heading", { name: "Explain this mix" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await expect(page.getByLabel("Mix map")).toHaveCount(0);
  await page.keyboard.press("Meta+e");
  await expect(page.getByLabel("Mix map")).toBeVisible();
  await page.keyboard.press("Meta+e");
  await expect(page.getByLabel("Timeline")).toBeVisible();
  // It is in the table of shortcuts.
  await page.getByRole("button", { name: /^Settings/ }).click();
  await page.getByRole("button", { name: "Shortcuts" }).click();
  await expect(page.getByText("Explain this mix")).toBeVisible();
});

// req: UI-15
test("the map shows who goes where, and a click lights what an element is connected to", async ({ page }) => {
  await openExplain(page);
  const id = await ids(page);
  const map = page.getByTestId("mix-map");
  // Tracks by family, the groups, the shared effects, the mix bus.
  for (const label of ["Tracks", "Groups", "Shared effects", "Lead vocal", "Drums", "Guitars"]) await expect(map.getByText(label, { exact: true }).first()).toBeVisible();
  for (const n of ["Kick", "Bass DI", "Lead Vox", "Drums", "Guitars", "Plate", "Echo", "mix"]) await expect(node(page, n)).toBeVisible();
  await expect(node(page, "Plate")).toContainText("plate reverb · 1.2 s");
  await expect(node(page, "Plate")).toContainText("4 in");
  // Left to right: a track, its group, the mix bus.
  const x = async (n: string) => (await node(page, n).boundingBox())!.x;
  expect(await x("Kick")).toBeLessThan(await x("Drums"));
  expect(await x("Drums")).toBeLessThan(await x("mix"));
  // One output per element, a send per feed, a sidechain per listener.
  const link = (l: string) => map.locator(`[data-link="${l}"]`);
  await expect(link(`o:${id("Kick")}`)).toHaveCount(1);
  await expect(link(`s:${id("Snare")}>${id("Plate")}`)).toHaveClass(/send/);
  await expect(link(`k:${id("Kick")}>${id("Bass DI")}`)).toHaveClass(/key/);
  await expect(map).not.toHaveClass(/has-sel/);
  // Lead Vox: its output, its two sends with their levels, the returns' way to the mix bus; the rest dims.
  await node(page, "Lead Vox").click();
  await expect(map).toHaveClass(/has-sel/);
  await expect(node(page, "Lead Vox")).toHaveClass(/is-sel/);
  for (const n of ["Plate", "Echo", "mix"]) await expect(node(page, n)).toHaveClass(/is-rel/);
  await expect(node(page, "Kick")).not.toHaveClass(/is-rel|is-sel/);
  await expect(link(`o:${id("Lead Vox")}`)).toHaveAttribute("data-lit", "true");
  await expect(link(`s:${id("Lead Vox")}>${id("Plate")}`)).toHaveAttribute("data-lit", "true");
  await expect(link(`o:${id("Kick")}`)).toHaveAttribute("data-lit", "false");
  await expect(map.getByText("−14 dB", { exact: true })).toBeVisible();
  await expect(map.getByText("−20 dB", { exact: true })).toBeVisible();
  // Bass DI: who it listens to. Plate: who feeds it.
  await node(page, "Bass DI").click();
  await expect(node(page, "Kick")).toHaveClass(/is-rel/);
  await expect(map.getByText("listens", { exact: true })).toBeVisible();
  await node(page, "Plate").click();
  for (const n of ["Snare", "Acoustic", "Lead Vox", "BVs"]) await expect(node(page, n)).toHaveClass(/is-rel/);
  await expect(node(page, "Rhodes")).not.toHaveClass(/is-rel/);
  // A second click on the selection goes back to the whole mix.
  await node(page, "Plate").click();
  await expect(map).not.toHaveClass(/has-sel/);
});

// req: UI-15
test("groups fold, a section shows what plays there, and a double click opens the element in the workspace", async ({ page }) => {
  await openExplain(page);
  const id = await ids(page);
  const map = page.getByTestId("mix-map");
  // Folded, Drums is one row with its count; Snare's send starts from it; the row unfolds it.
  await page.getByRole("button", { name: "Fold Drums" }).click();
  await expect(node(page, "Kick")).toHaveCount(0);
  const stub = node(page, `fold:${id("Drums")}`);
  await expect(stub).toHaveText(/4 tracks/);
  await expect(node(page, "Drums")).toContainText("4 tracks · folded");
  await expect(map.locator(`[data-link="s:fold:${id("Drums")}>${id("Plate")}"]`)).toHaveCount(1);
  await expect(page.getByRole("button", { name: "Unfold all" })).toBeVisible();
  // A name in the text that is inside a folded group opens it.
  await text(page).getByRole("button", { name: "Kick", exact: true }).first().click();
  await expect(node(page, "Kick")).toHaveClass(/is-sel/);
  await page.getByRole("button", { name: "Back to the whole mix" }).click();
  // Every chorus counts as one section: what plays only there is tagged, what is silent dims.
  const chorus = page.getByRole("button", { name: "Chorus", exact: true });
  await chorus.first().click();
  for (const c of await chorus.all()) await expect(c).toHaveAttribute("aria-pressed", "true");
  for (const n of ["Gtr Left", "Gtr Right", "BVs"]) await expect(node(page, n)).toContainText("only here");
  await expect(node(page, "Lead Vox")).toContainText("block on");
  await expect(node(page, "Rhodes")).toHaveClass(/is-off/);
  await expect(node(page, "Kick")).not.toHaveClass(/is-off/);
  await expect(page.getByRole("heading", { name: "Chorus" })).toBeVisible();
  await expect(page.getByLabel("Explanation").getByText("3 times · 0:48, 1:52, 2:48")).toBeVisible();
  await page.getByRole("button", { name: "Whole song" }).click();
  await expect(node(page, "Rhodes")).not.toHaveClass(/is-off/);
  // Double click: out of the view, with the element selected in the workspace.
  await node(page, "Bass DI").dblclick();
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await expect(page.getByLabel("Inspector", { exact: true })).toContainText("Bass DI");
});

// req: UI-16, ANA-20
test("the text tells the whole mix in order, with the levels the core measures", async ({ page }) => {
  await openExplain(page);
  const t = text(page);
  // The levels are measured in the background: the part says so, then shows them; the rest does not wait.
  await expect(t.getByRole("status")).toContainText("Measuring how loud each family");
  await expect(t.getByText("12 tracks").first()).toBeVisible();
  const balance = t.getByRole("region", { name: "Who is in front" });
  await expect(balance.getByRole("listitem").first()).toHaveAccessibleName(/^Lead vocal: −\d+\.\d LUFS$/);
  await expect(balance.getByRole("listitem")).toHaveCount(7);
  await expect(t.getByRole("status")).toHaveCount(0);
  await expect(node(page, "mix")).toContainText(/−\d+\.\d LUFS/);
  // The parts, always in this order.
  await expect(t.getByRole("heading")).toHaveText(["In short", "How it is built", "Who is in front", "Left to right", "Space", "Who reacts to whom", "What changes along the song", "Reasons"]);
  await expect(t.getByRole("region", { name: "In short" })).toContainText("12 tracks reach the mix bus through 2 groups and share 2 effects.");
  await expect(t.getByRole("region", { name: "How it is built" })).toContainText("4 tracks send it a copy of themselves");
  await expect(t.getByRole("region", { name: "Space" })).toContainText("is the wettest in");
  await expect(t.getByRole("region", { name: "Who reacts to whom" })).toContainText("each time Kick plays, Bass DI is turned down for a moment");
  await expect(t.getByRole("region", { name: "What changes along the song" })).toContainText("play only here");
  // Reasons are the assistant's own notes, quoted.
  await expect(t.getByRole("region", { name: "Reasons" })).toContainText("Ducks 2 dB on every kick (sidechain) to free the low end without thinning the bass.");
  // The navigation jumps to a part.
  await page.getByRole("tab", { name: "Reasons" }).click();
  await expect(t.getByRole("heading", { name: "Reasons" })).toBeInViewport();
  // A term explains itself in one sentence, on hover and on focus.
  await page.getByRole("tab", { name: "In short" }).click();
  const term = t.getByRole("note").filter({ hasText: /^groups/ }).first();
  await expect(term.getByRole("tooltip")).toBeHidden();
  await term.hover();
  await expect(term.getByRole("tooltip")).toHaveText("A group gathers several tracks into one channel, so they can be treated and moved together.");
});

// req: UI-16
test("a name in the text selects the element, whose story replaces the text; the text copies as plain text", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "clipboard", { value: { writeText: (s: string) => ((window as unknown as { __copied: string }).__copied = s, Promise.resolve()) }, configurable: true });
  });
  await openExplain(page);
  const t = text(page);
  await t.getByRole("region", { name: "Who reacts to whom" }).getByRole("button", { name: "Bass DI" }).click();
  await expect(node(page, "Bass DI")).toHaveClass(/is-sel/);
  const side = page.getByLabel("Explanation");
  await expect(side.getByRole("heading", { name: "Bass DI" })).toBeVisible();
  await expect(side.getByText("bass", { exact: true })).toBeVisible();
  await expect(t.getByRole("heading")).toHaveText(["Where it goes", "Its place in the mix", "Who it reacts to", "What happens to it, in order", "Why it is set this way"]);
  await expect(t.getByRole("region", { name: "Where it goes" })).toContainText("Bass DI goes straight to the mix bus");
  // The chain in signal order, each block with what it does; the bypassed ones named under it.
  const chain = t.getByRole("region", { name: "What happens to it, in order" });
  await expect(chain.getByRole("listitem")).toHaveCount(5);
  await expect(chain.getByRole("listitem").nth(0)).toContainText("High-pass40 HzRemoves what is below 40 Hz");
  await expect(chain.getByRole("listitem").nth(2)).toContainText("Listens to Kick through a sidechain");
  await expect(chain.getByRole("listitem").nth(4)).toContainText("Automated by hand");
  await expect(chain).toContainText("Off on this track: Saturation.");
  await expect(t.getByRole("region", { name: "Why it is set this way" })).toContainText("Ducks 2 dB on every kick");
  // It follows the project: a fader moved elsewhere is in the text at once.
  const id = await ids(page);
  await mock(page, `setParam("${id("Bass DI")}", "fader", -11, "Bass DI · fader")`);
  await expect(t.getByRole("region", { name: "Its place in the mix" })).toContainText("Fader at −11.0 dB");
  // Solo is the workspace's own switch; "Open in inspector" leaves the view.
  await side.getByRole("button", { name: "Solo" }).click();
  await expect(side.getByRole("button", { name: "Solo" })).toHaveAttribute("aria-pressed", "true");
  await side.getByRole("button", { name: "Solo" }).click();
  // Copy as text: the story as plain text, numbered.
  await side.getByRole("button", { name: "Copy as text" }).click();
  await expect(page.getByText("The explanation is on the clipboard as plain text.")).toBeVisible();
  const copied = await page.evaluate(() => (window as unknown as { __copied: string }).__copied);
  expect(copied).toContain("Bass DI\n\nWHERE IT GOES\nBass DI goes straight to the mix bus");
  expect(copied).toContain("1. High-pass (40 Hz): Removes what is below 40 Hz");
  // A return has its own story.
  await page.getByRole("button", { name: "Back to the whole mix" }).click();
  await node(page, "Plate").click();
  await expect(t.getByRole("heading")).toHaveText(["What it is", "Who sends here", "What it does for the mix"]);
  await expect(t.getByRole("region", { name: "Who sends here" }).getByRole("listitem")).toHaveCount(4);
  await side.getByRole("button", { name: "Open in inspector" }).click();
  await expect(page.getByLabel("Inspector", { exact: true })).toContainText("Plate");
});

// req: UI-14, UI-7
test("17 · explain: the view at the reference and minimum sizes, readable by keyboard and assistive tech", async ({ page }) => {
  await openExplain(page);
  await expect(text(page).getByRole("region", { name: "Who is in front" }).getByRole("listitem").first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth)).toBe(false);
  // The map fits the reference window without scrolling.
  expect(await page.evaluate(() => { const s = document.querySelector(".xscroll")!; return s.scrollHeight <= s.clientHeight && s.scrollWidth <= s.clientWidth; })).toBe(true);
  await page.screenshot({ path: "test-results/visual/17b-explain.png" });
  const r = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
  expect(r.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" | ")}`)).toEqual([]);
  await node(page, "Lead Vox").click();
  await page.screenshot({ path: "test-results/visual/17c-explain-track.png" });
  await page.getByRole("button", { name: "Chorus", exact: true }).first().click();
  await page.screenshot({ path: "test-results/visual/17f-explain-section.png" });
  // Keyboard: a box is a button, Enter selects it.
  await page.getByRole("button", { name: "Whole song" }).click();
  await node(page, "Kick").focus();
  await page.keyboard.press("Enter");
  await expect(node(page, "Kick")).toHaveAttribute("aria-pressed", "true");
  await page.setViewportSize({ width: 1280, height: 720 });
  await expect(page.getByTestId("mix-map")).toHaveAttribute("data-mode", "compact");
  expect(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth)).toBe(false);
  await page.keyboard.press("Escape");
  await page.screenshot({ path: "test-results/visual/17i-explain-min.png" });
});
