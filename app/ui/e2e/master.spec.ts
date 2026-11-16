import { expect, test, type Page } from "@playwright/test";
import { expectColumnsAligned } from "./tables";

// The master phase (F8 gate) on the mock backend: bounce as an explicit step (MDN-1), master workspace with the
// destination and equal-loudness A/B (MAS-2, MAS-5), modules (MAS-1), master AI with partial application (MAS-6),
// export with metadata and report (MAS-7, MAS-9), back to Mix (MDN-3), out-of-date mixdown (MDN-2).

/** A block of the chain: its row in the rack, opened by the button with the block's name (UI-7). */
const moduleCard = (page: Page, name: string) => page.locator(".rrow").filter({ has: page.getByRole("button", { name, exact: true }) });
/** The names of the rack's rows, top to bottom. */
const rackNames = (page: Page) => page.getByLabel("Master chain").locator(".rrow:not(.is-drag) .rname").allTextContents();

async function toMaster(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await page.getByRole("radio", { name: "Master" }).click();
  const dlg = page.getByRole("dialog", { name: "Bounce & go to Master" });
  await expect(dlg).toBeVisible();
  await dlg.getByRole("button", { name: "Bounce & go to Master" }).click();
  await expect(page.getByRole("dialog", { name: /Mixdown \d+ is ready/ })).toBeVisible({ timeout: 10_000 });
  await expect(page.getByTestId("bounce-lufs")).toContainText("LUFS");
  await page.getByRole("button", { name: "Open Master" }).click();
  await expect(page.getByLabel("Master chain")).toBeVisible();
}

// req: MDN-1, MDN-3, MAS-1, MAS-2, MAS-5, UI-1
test("bounce, master workspace, destination, A/B, back to Mix", async ({ page }) => {
  await toMaster(page);
  await expect(page.locator(".sw.app")).toHaveAttribute("data-phase", "master");
  await expect(page.getByTestId("destination")).toContainText("Streaming");
  await page.getByRole("button", { name: "Destination" }).click();
  await page.getByRole("menuitem", { name: /Loud · −9|Loud · -9/ }).click();
  await expect(page.getByTestId("destination")).toContainText("Loud");
  await expect(page.getByTestId("master-lufs")).toHaveText("−9.1");
  // Modules: open the compressor, switch it on.
  await page.getByRole("button", { name: "Compressor", exact: true }).click();
  await page.getByRole("button", { name: "Compressor on" }).first().click();
  await expect(moduleCard(page, "Compressor")).not.toHaveClass(/is-off/);
  // A/B at equal loudness.
  await page.getByRole("radio", { name: "Mixdown" }).click();
  await expect(page.getByRole("radio", { name: "Mixdown" })).toHaveAttribute("aria-checked", "true");
  await page.getByRole("radio", { name: "Mix", exact: true }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await expect(page.locator(".sw.app")).toHaveAttribute("data-phase", "mix");
});

// req: MAS-6, AI-2
test("master AI: proposal, A/B with B and ⇧B, apply only some modules", async ({ page }) => {
  await toMaster(page);
  await page.getByRole("button", { name: "Master with AI" }).click();
  const panel = page.getByRole("complementary", { name: "Master proposal" });
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel).toContainText("Distance to reference");
  // 9c, 9o: the proposal's A/B follows the listening mode; B switches source, ⇧B the mode.
  await expect(panel.getByTestId("proposal-level")).toContainText("B switches · ⇧B same loudness");
  await page.locator("body").press("b");
  await expect(panel.getByRole("radio", { name: "Proposal" })).toHaveAttribute("aria-checked", "true");
  await expect(page.getByTestId("lrow-Proposal")).toHaveAttribute("aria-current", "true");
  await page.locator("body").press("Shift+B");
  await expect(panel.getByTestId("proposal-level")).toContainText(/current master.*⇧B real level/);
  await page.locator("body").press("b");
  await expect(panel.getByRole("radio", { name: "Current master" })).toHaveAttribute("aria-checked", "true");
  // 15f: what the render of the plan measured, and the proposed chain in the rack (the compressor moved first).
  await expect(panel.getByTestId("proposal-measured")).toContainText("Limiter over 3 dB");
  await expect(panel.getByTestId("proposal-measured")).toContainText("Peaks above body");
  expect((await rackNames(page)).slice(0, 3)).toEqual(["Input", "Compressor", "EQ"]);
  await expect(page.getByLabel("Master chain")).toContainText("as proposed");
  await expect(page.getByTestId("add-block")).toHaveCount(0);
  // Every entry has its tick: the clipper is left out.
  await panel.locator('[data-name="Clipper"]').getByRole("checkbox").click();
  await panel.getByRole("button", { name: /Apply selected/ }).click();
  await expect(panel).toBeHidden();
  await expect(moduleCard(page, "Match EQ")).toContainText("50 %");
  await expect(moduleCard(page, "Compressor")).toContainText("parallel");
  await expect(moduleCard(page, "Clipper")).toContainText("Off");
  // The sections the plan offsets show in the lane under the waveform.
  await expect(page.getByTestId("section-offsets")).toContainText("+0.5 dB");
});

// req: MAS-15, MAS-11, MAS-13, UI-7
test("the rack: add, move, switch, reset and remove blocks; clipper and compressor", async ({ page }) => {
  await toMaster(page);
  const rack = page.getByLabel("Master chain");
  expect(await rackNames(page)).toEqual(["Input", "EQ", "Match EQ", "Compressor", "Multiband", "Width", "Saturation", "Clipper", "Limiter", "Dither"]);
  await expect(page.getByTestId("add-block")).toContainText("10 of 12");
  // 15i: "Add module" is always in sight; the menu says what cannot be added and why.
  await page.getByTestId("add-block").click();
  const menu = page.getByRole("menu");
  await expect(menu).toContainText("Match EQ and Width: only one");
  await expect(menu.getByRole("menuitem", { name: "Width" })).toHaveCount(0);
  await menu.getByRole("menuitem", { name: "EQ", exact: true }).click();
  // The new block lands before the limiter, off, and opens in the editor; two of a kind are numbered.
  expect((await rackNames(page)).slice(-3)).toEqual(["EQ 2", "Limiter", "Dither"]);
  await expect(page.getByRole("region", { name: "EQ 2" })).toBeVisible();
  await expect(moduleCard(page, "EQ 2")).toHaveClass(/is-off/);
  await expect(page.getByTestId("add-block")).toContainText("11 of 12");
  // Two EQs already: the menu no longer offers a third.
  await page.getByTestId("add-block").click();
  await expect(page.getByRole("menu")).toContainText("EQ: two already");
  await page.keyboard.press("Escape");
  // 15b: ⌥↑ moves the selected block; one master step each.
  await rack.getByRole("button", { name: "EQ 2", exact: true }).focus();
  await page.keyboard.press("Alt+ArrowUp");
  await page.keyboard.press("Alt+ArrowUp");
  expect((await rackNames(page)).slice(-6, -1)).toEqual(["Width", "EQ 2", "Saturation", "Clipper", "Limiter"]);
  await page.locator("body").press("ControlOrMeta+z");
  expect((await rackNames(page)).slice(-4)).toEqual(["EQ 2", "Clipper", "Limiter", "Dither"]);
  // 15j: the menu of a block removes it; the fixed blocks have none.
  await rack.getByRole("button", { name: "EQ 2: move or more" }).press("Enter");
  await page.getByRole("menuitem", { name: "Remove" }).click();
  await expect(rack.getByRole("button", { name: "EQ 2", exact: true })).toHaveCount(0);
  await expect(page.getByTestId("add-block")).toContainText("10 of 12");
  // MAS-11: the clipper says how much it takes off the tallest peaks.
  await rack.getByRole("button", { name: "Clipper", exact: true }).click();
  const clip = page.getByRole("region", { name: "Clipper" });
  await clip.getByRole("button", { name: "Clipper on" }).click();
  await expect(moduleCard(page, "Clipper")).toContainText("Up to 2.0 dB");
  await expect(clip).toContainText("under the tallest peak");
  await expect(clip.getByTestId("clipper-work")).toContainText("% of the song");
  await expect(page.getByRole("complementary", { name: "Loudness" })).toContainText("Clipper");
  // MAS-13: the compressor has a mix and a detector filter.
  await rack.getByRole("button", { name: "Compressor", exact: true }).click();
  const comp = page.getByRole("region", { name: "Compressor" });
  await expect(comp).toContainText("Listens above");
  await expect(comp.getByRole("slider", { name: /mix/i })).toBeVisible();
  // "Reset" puts the free blocks back in the order of a new project.
  await rack.getByRole("button", { name: "Compressor: move or more" }).press("Enter");
  await page.getByRole("menuitem", { name: "Move up" }).click();
  expect((await rackNames(page)).slice(1, 4)).toEqual(["EQ", "Compressor", "Match EQ"]);
  await rack.getByRole("button", { name: "Reset", exact: true }).click();
  expect((await rackNames(page)).slice(1, 4)).toEqual(["EQ", "Match EQ", "Compressor"]);
});

// req: MAS-12
test("multiband: two to six bands, split and merge, a band on its own, solo", async ({ page }) => {
  await toMaster(page);
  await page.getByLabel("Master chain").getByRole("button", { name: "Multiband", exact: true }).click();
  const mb = page.getByRole("region", { name: "Multiband" });
  await expect(mb.getByTestId("band-count")).toHaveText("3 bands");
  await expect(mb.getByRole("group", { name: "Bands" }).locator(".brow")).toHaveCount(3);
  await mb.getByRole("button", { name: "One band more" }).click();
  await mb.getByRole("button", { name: "One band more" }).click();
  await expect(mb.getByTestId("band-count")).toHaveText("5 bands");
  await expect(mb.getByRole("group", { name: "Bands" })).toContainText("Presence");
  await expect(mb.getByRole("slider", { name: /Crossover/ })).toHaveCount(4);
  await expect(moduleCard(page, "Multiband")).toContainText("Off");
  await mb.getByRole("button", { name: "Multiband on" }).click();
  await expect(moduleCard(page, "Multiband")).toContainText("5 bands");
  // A band is selected by its name and has its own controls; solo is for listening.
  await mb.getByRole("button", { name: "Presence", exact: true }).click();
  await expect(mb).toContainText("Presence selected");
  await mb.getByRole("button", { name: "Solo Presence" }).click();
  await expect(mb.getByRole("button", { name: "Solo Presence" })).toHaveAttribute("aria-pressed", "true");
  for (let i = 0; i < 3; i++) await mb.getByRole("button", { name: "One band less" }).click();
  await expect(mb.getByTestId("band-count")).toHaveText("2 bands");
  await expect(mb.getByRole("button", { name: "One band less" })).toBeDisabled();
});

// req: MAS-14
test("section offsets: a section opens its offsets, which show under the waveform", async ({ page }) => {
  await toMaster(page);
  await page.getByRole("button", { name: "Offsets of Chorus" }).first().click();
  const ed = page.getByTestId("section-editor");
  await expect(ed).toContainText("Offsets added to the song-wide master");
  const level = ed.locator(".fx", { hasText: "More or less level" }).locator("input, [role=spinbutton], .field").first();
  await level.dblclick();
  await page.keyboard.type("0.5");
  await page.keyboard.press("Enter");
  // Every chorus gets it ("All"), and the lane says so.
  await expect(page.getByTestId("section-offsets").locator(".so")).toHaveCount(3);
  await expect(page.getByTestId("section-offsets")).toContainText("+0.5 dB");
  await ed.getByRole("button", { name: "Clear" }).click();
  await expect(page.getByTestId("section-offsets")).toHaveCount(0);
  await ed.getByRole("button", { name: "Done" }).click();
  await expect(page.getByRole("region", { name: "Limiter" })).toBeVisible();
});

// req: MAS-6
test("a target that asks too much of the mix says so and offers the ways out", async ({ page }) => {
  await toMaster(page);
  await expect(page.getByTestId("strained")).toHaveCount(0);
  await page.getByRole("button", { name: "Destination" }).click();
  await page.getByRole("menuitem", { name: /Loud · −9|Loud · -9/ }).click();
  const b = page.getByTestId("strained");
  await expect(b).toContainText("too peaky for −9.0 LUFS");
  await expect(page.getByTestId("limiter-work")).toContainText("% of the song");
  // The way back to the mix writes the request for the author and sends nothing.
  await b.getByRole("button", { name: /Tame those peaks in Mix/ }).click();
  await expect(page.locator(".sw.app")).toHaveAttribute("data-phase", "mix");
  await expect(page.getByRole("textbox", { name: /assistant|request/i }).first()).toHaveValue(/Lower the tallest peaks/);
});

// req: MAS-4
test("the waveform of a loud master keeps its shape: the body under a band of peaks", async ({ page }) => {
  await toMaster(page);
  await page.getByRole("button", { name: "Destination" }).click();
  await page.getByRole("menuitem", { name: /Loud · −9|Loud · -9/ }).click();
  await expect(page.getByTestId("master-lufs")).toHaveText("−9.1");
  // The heights of a path's upper edge, to the pixel.
  const heights = async (id: string) => {
    const d = (await page.getByTestId(id).getAttribute("d")) ?? "";
    const pts = d.slice(1, -1).split("L").map((p) => Math.round(Number(p.split(" ")[1])));
    return new Set(pts.slice(0, pts.length / 2));
  };
  await expect.poll(async () => (await heights("master-body")).size).toBeGreaterThan(8);
  // At this loudness most peaks sit at the ceiling: their band is nearly flat, and the body is what tells the song.
  const [peaks, body] = [await heights("master-peaks"), await heights("master-body")];
  expect(body.size).toBeGreaterThan(peaks.size);
  expect(Math.min(...body)).toBeGreaterThan(Math.min(...peaks) + 4);
  await expect(page.getByLabel("Mixdown", { exact: true })).toContainText("line: peaks");
  await page.screenshot({ path: "test-results/visual/09w-loud-waveform.png" });
});

// req: MAS-7, MAS-9, MDN-2
test("export with metadata and report; a mix change makes the mixdown out of date", async ({ page }) => {
  await toMaster(page);
  await page.getByRole("button", { name: "Export" }).click();
  const dlg = page.getByRole("dialog", { name: "Export master" });
  await dlg.getByLabel("Title").fill("Confidence");
  await dlg.getByLabel("ISRC").fill("IT-ABC-26-00001");
  await dlg.getByLabel("ISRC").blur();
  await expect(page.getByTestId("isrc-state")).toContainText("Valid ISRC format");
  await dlg.getByRole("checkbox", { name: "FLAC" }).click();
  await expect(dlg.getByRole("button", { name: /Export 2 files/ })).toBeVisible();
  await dlg.getByRole("button", { name: /Export 2 files/ }).click();
  const rep = page.getByRole("dialog", { name: "Export complete" });
  await expect(rep).toBeVisible({ timeout: 10_000 });
  await expect(rep.getByRole("table", { name: "Export report" })).toContainText(".flac");
  await expectColumnsAligned(rep.getByRole("table", { name: "Export report" }));
  await rep.getByRole("button", { name: "Done" }).click();
  // Back to Mix, change the mix: Master opens straight on the master already made, which says the mixdown is out
  // of date and offers to bounce again keeping its settings.
  await page.getByRole("radio", { name: "Mix", exact: true }).click();
  await page.getByTestId("strip-Kick").locator(".fader").dblclick();
  await page.getByRole("radio", { name: "Master" }).click();
  await expect(page.getByRole("dialog", { name: "Bounce & go to Master" })).toHaveCount(0);
  await expect(page.getByLabel("Master chain")).toBeVisible();
  await expect(page.getByText("The mix has changed since this mixdown")).toBeVisible();
  await page.getByRole("button", { name: "Bounce again and keep these settings" }).click();
  await expect(page.getByRole("dialog", { name: "Bounce & go to Master" })).toBeVisible();
});

// req: MDN-3, UI-3
test("back and forth between Mix and Master without a new bounce; space plays and pauses in both", async ({ page }) => {
  await toMaster(page);
  const play = () => page.getByRole("group", { name: "Transport" }).getByRole("button", { name: /^(Play|Pause)$/ });
  await page.locator("body").press("Space");
  await expect(play()).toHaveAccessibleName("Pause");
  await page.locator("body").press("Space");
  await expect(play()).toHaveAccessibleName("Play");
  await page.getByRole("radio", { name: "Mix", exact: true }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await page.locator("body").press("Space");
  await expect(play()).toHaveAccessibleName("Pause");
  await page.locator("body").press("Space");
  await page.getByRole("radio", { name: "Master" }).click();
  await expect(page.getByRole("dialog", { name: "Bounce & go to Master" })).toHaveCount(0);
  await expect(page.getByLabel("Master chain")).toBeVisible();
});

type PrepMock = { __stemwiseMock: { simulatePrep(stepMs: number, failure?: string | null, remeasureMs?: number): void } };
const simulatePrep = (page: Page, stepMs: number, failure: string | null = null, remeasureMs = 0) =>
  page.evaluate(([a, b, c]) => (window as unknown as PrepMock).__stemwiseMock.simulatePrep(a as number, b as string | null, c as number), [stepMs, failure, remeasureMs] as const);

async function bounceToMaster(page: Page) {
  await page.getByRole("radio", { name: "Master" }).click();
  await page.getByRole("dialog", { name: "Bounce & go to Master" }).getByRole("button", { name: "Bounce & go to Master" }).click();
  await page.getByRole("button", { name: "Open Master" }).click({ timeout: 10_000 });
}

// req: MDN-1, MDN-3, MAS-2, UI-7
test("preparing the master: steps under an overlay, Esc back to Mix keeps it going, a toast when it is ready", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await simulatePrep(page, 900);
  await bounceToMaster(page);
  // 9d: the overlay with its steps; the workspace underneath is not usable yet.
  const overlay = page.getByTestId("prep-overlay");
  await expect(overlay).toBeVisible();
  await expect(overlay).toContainText("Preparing the master");
  await expect(overlay).toContainText(/Mixdown \d+ · 3:42 · 48 kHz · 1 reference/);
  await expect(overlay.getByTestId("prep-steps")).toContainText("From the bounce");
  await expect(overlay.getByTestId("prep-steps")).toContainText("Converting Reference A to 48 kHz");
  await expect(page.getByLabel("Master chain")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Export" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Back to Mix" })).toBeFocused();
  // Space does nothing while preparing.
  await page.keyboard.press("Space");
  await expect(page.getByRole("group", { name: "Transport" }).getByRole("button", { name: "Play" })).toBeVisible();
  // 9f: Esc goes back to Mix; the Master segment shows the ring.
  await page.keyboard.press("Escape");
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await expect(page.getByTestId("prep-ring")).toBeVisible();
  await expect(page.getByRole("radio", { name: /Master/ })).toHaveAttribute("title", /Preparing the master · \d+ % · keeps going while you mix/);
  // 9g: ready in Mix, the toast opens Master at once.
  const toast = page.locator(".toast").filter({ hasText: "Master is ready" });
  await expect(toast).toBeVisible({ timeout: 10_000 });
  await toast.getByRole("button", { name: "Open Master" }).click();
  await expect(page.getByLabel("Master chain")).toBeVisible();
  await expect(page.getByTestId("prep-overlay")).toHaveCount(0);
});

// req: MDN-1, MAS-5
test("preparing the master: an unreadable mixdown blocks and offers a new bounce; an unreadable reference is left out", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await simulatePrep(page, 250, "mixdown_unreadable");
  await bounceToMaster(page);
  // 9j
  const err = page.getByTestId("prep-error");
  await expect(err).toBeVisible();
  await expect(err).toContainText("The master can’t be prepared");
  await expect(err).toContainText("The file ends early");
  await simulatePrep(page, 250, "reference");
  await err.getByRole("button", { name: "Bounce again" }).click();
  await page.getByRole("dialog", { name: "Bounce & go to Master" }).getByRole("button", { name: "Bounce & go to Master" }).click();
  await page.getByRole("button", { name: "Open Master" }).click({ timeout: 10_000 });
  // 9h → 9i: Master opens with the banner naming the reference left out.
  await expect(page.getByLabel("Master chain")).toBeVisible({ timeout: 10_000 });
  const banner = page.getByTestId("skipped-references");
  await expect(banner).toContainText("Reference A can’t be read");
  await banner.getByRole("button", { name: "Not now" }).click();
  await expect(banner).toHaveCount(0);
});

// req: MAS-2
test("a master change is measured again in place, without the overlay", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await simulatePrep(page, 100, null, 1500);
  await bounceToMaster(page);
  await expect(page.getByLabel("Master chain")).toBeVisible({ timeout: 10_000 });
  await page.getByRole("button", { name: "Compressor", exact: true }).click();
  await page.getByRole("button", { name: "Compressor on" }).first().click();
  // 9k: the Loudness card says it is measuring; no overlay; Master with AI waits.
  await expect(page.getByTestId("measuring-note")).toBeVisible();
  await expect(page.getByRole("complementary", { name: "Loudness" })).toContainText("before the change");
  await expect(page.getByTestId("prep-overlay")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Master with AI" })).toBeDisabled();
  await expect(page.getByTestId("measuring-note")).toHaveCount(0, { timeout: 5000 });
  await expect(page.getByRole("button", { name: "Master with AI" })).toBeEnabled();
});

// req: MAS-2, UI-7
test("a new target waits under an overlay until the level is in place; the transport still works, Esc goes back to Mix", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await simulatePrep(page, 100, null, 2500);
  await bounceToMaster(page);
  await expect(page.getByLabel("Master chain")).toBeVisible({ timeout: 10_000 });
  await expect(page.getByTestId("master-lufs")).toHaveText("−14.1");
  await page.getByRole("button", { name: "Destination" }).click();
  await page.getByRole("menuitem", { name: /Loud · −9/ }).click();
  // 9v: the overlay names the new target and the one before; the workspace behind still shows the old level.
  const overlay = page.getByTestId("retarget-overlay");
  await expect(overlay).toBeVisible();
  await expect(overlay).toContainText("Bringing the master to the new target");
  await expect(page.getByTestId("retarget-sub")).toHaveText("Loud · −9 LUFS · −1 dBTP · was −14 LUFS, −1 dBTP");
  await expect(page.getByTestId("retarget-steps")).toContainText("Finding the gain for −9 LUFS");
  await expect(page.getByTestId("master-lufs")).toHaveText("−14.1");
  await expect(page.getByTestId("measuring-note")).toHaveCount(0);
  // The button of this overlay: on a slow machine the preparation overlay may still be fading out behind it.
  await expect(overlay.getByRole("button", { name: "Back to Mix" })).toBeFocused();
  await expect(page.getByRole("button", { name: "Export" })).toBeDisabled();
  // Undo waits too (its group is inert, and so is the key); the transport does not.
  await page.keyboard.press("ControlOrMeta+z");
  await expect(page.getByTestId("destination")).toContainText("Loud");
  await page.keyboard.press("Space");
  await expect(page.getByRole("group", { name: "Transport" }).getByRole("button", { name: "Pause" })).toBeVisible();
  await page.keyboard.press("Space");
  // Esc: back to Mix with the ring, and the work goes on.
  await page.keyboard.press("Escape");
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await expect(page.getByRole("radio", { name: /Master/ })).toHaveAttribute("title", /Bringing the master to the new target · \d+ % · keeps going while you mix/);
  await page.getByRole("radio", { name: /Master/ }).click();
  // Done: the overlay goes and the workspace is at the new level, the focus back on the destination.
  await expect(overlay).toHaveCount(0, { timeout: 8000 });
  await expect(page.getByTestId("master-lufs")).toHaveText("−9.1");
  await expect(page.getByRole("complementary", { name: "Loudness" })).toContainText("On target");
  await expect(page.getByRole("button", { name: "Export" })).toBeEnabled();
  // A target already measured comes back at once, with no overlay (here: undo).
  await simulatePrep(page, 100, null, 0);
  await page.keyboard.press("ControlOrMeta+z");
  await expect(page.getByTestId("master-lufs")).toHaveText("−14.1");
  await expect(overlay).toHaveCount(0);
});

// req: MAS-15
test("the master rack: a block dragged by its grip moves in one step, with no error", async ({ page }) => {
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  await toMaster(page);
  const rack = page.getByLabel("Master chain").first();
  const before = await rackNames(page);
  const name = before.find((n, i) => i > 1 && !["Limiter", "Dither"].includes(n))!;
  const first = before[1];
  const grip = rack.getByRole("button", { name: `${name}: move or more` });
  const target = (await rack.getByRole("button", { name: first, exact: true }).boundingBox())!;
  await grip.hover();
  await page.mouse.down();
  await page.mouse.move(target.x + 10, target.y + 2, { steps: 6 });
  await page.mouse.up();
  await expect.poll(async () => (await rackNames(page)).indexOf(name)).toBeLessThan(before.indexOf(name));
  expect(errors.filter((e) => /while rendering a different component/.test(e))).toEqual([]);
});

// req: TRN-1, MAS-5
test("scrub on the master waveform: the playhead follows the drag", async ({ page }) => {
  await toMaster(page);
  const wave = (await page.getByTestId("master-wave").boundingBox())!;
  const head = async () => (await page.getByTestId("master-playhead").boundingBox())!.x;
  await page.mouse.move(wave.x + 30, wave.y + wave.height / 2);
  await page.mouse.down();
  await page.mouse.move(wave.x + 300, wave.y + wave.height / 2, { steps: 8 });
  await expect.poll(head).toBeGreaterThan(wave.x + 280);
  await page.mouse.up();
  await expect(page.getByTestId("position")).not.toHaveText("0:00.00");
});

type RefMock = { __stemwiseMock: { simulateReferences(refs: { id: string; name: string; lufs: number; true_peak_db?: number }[]): void } };
const references = (page: Page, refs: { id: string; name: string; lufs: number; true_peak_db?: number }[]) =>
  page.evaluate((r) => (window as unknown as RefMock).__stemwiseMock.simulateReferences(r), refs);
const row = (page: Page, name: string) => page.getByTestId(`lrow-${name}`);

/** Custom destination at `lufs` from the Loudness card. */
async function customTarget(page: Page, lufs: string) {
  // The workspace is inert while the master is prepared (9d).
  await expect(page.locator(".scrim.prep")).toHaveCount(0, { timeout: 10_000 });
  await page.getByRole("button", { name: "Destination" }).click();
  await page.getByRole("menuitem", { name: /^Custom/ }).click();
  await page.getByRole("slider", { name: "Target loudness" }).click();
  await page.getByLabel("Target loudness value").fill(lufs);
  await page.getByLabel("Target loudness value").press("Enter");
}

// req: MAS-5, UI-3
test("listening: the master at its real level by default, the same loudness on demand with the trim explained", async ({ page }) => {
  await toMaster(page);
  // 9l: real level by default; every source says where it sits against the master.
  await expect(page.getByRole("radio", { name: "Real level" })).toHaveAttribute("aria-checked", "true");
  await expect(page.getByTestId("listen-mode")).toHaveText("Real level");
  await expect(row(page, "Master")).toContainText("as exported");
  await expect(row(page, "Mixdown")).toContainText("4.5 LU quieter");
  await expect(row(page, "Reference A")).toContainText("1.2 LU louder");
  await expect(row(page, "Master")).toHaveAttribute("aria-current", "true");
  await page.getByRole("radio", { name: "Mixdown" }).click();
  await expect(row(page, "Mixdown")).toHaveAttribute("aria-current", "true");
  // 9n: ⇧B switches to the same loudness; raising the mixdown would clip, so everything plays 2.5 dB lower.
  await page.locator("body").press("Shift+B");
  await expect(page.getByRole("radio", { name: "Same loudness" })).toHaveAttribute("aria-checked", "true");
  await expect(page.getByTestId("listen-mode")).toHaveText("Same loudness");
  await expect(row(page, "Master")).toContainText("lowered 2.5 dB");
  await expect(row(page, "Mixdown")).toContainText("raised 2.0 dB");
  await expect(row(page, "Reference A")).toContainText("lowered 3.7 dB");
  await expect(page.getByTestId("trim-note")).toContainText("Raising the mixdown 4.5 dB would push its peaks over −1 dBTP, so everything plays 2.5 dB lower");
  // Back with a click; B alone (the proposal's A/B) does not change the mode.
  await page.getByRole("radio", { name: "Real level" }).click();
  await expect(page.getByTestId("listen-mode")).toHaveText("Real level");
  await expect(page.getByTestId("trim-note")).toHaveCount(0);
  await page.locator("body").press("b");
  await expect(page.getByTestId("listen-mode")).toHaveText("Real level");
  // The mode holds across phases while the project is open.
  await page.locator("body").press("Shift+B");
  await page.getByRole("radio", { name: "Mix", exact: true }).click();
  await page.getByRole("radio", { name: "Master" }).click();
  await expect(page.getByTestId("listen-mode")).toHaveText("Same loudness");
});

// req: MAS-2, MAS-9
test("target warning: much quieter than the references, keep, warn again, match; in the export and its report", async ({ page }) => {
  await toMaster(page);
  await expect(page.getByTestId("target-warning")).toHaveCount(0);
  await references(page, [
    { id: "ref1", name: "learn-to-fly.wav", lufs: -9.6, true_peak_db: -0.3 },
    { id: "ref2", name: "other.wav", lufs: -11.2, true_peak_db: -0.5 },
  ]);
  // D31: two references shorten the A/B segments.
  await expect(page.getByRole("radio", { name: "learn-to-fly.wav" })).toHaveText("Ref A");
  await customTarget(page, "-18");
  // 9r: compared with the quietest reference; the select and the far references turn amber.
  const warn = page.getByTestId("target-warning");
  await expect(warn).toContainText("Much quieter than your references");
  await expect(warn).toContainText("6.8 LU below Reference B (−11.2 LUFS) and 8.4 LU below Reference A (−9.6 LUFS)");
  await expect(warn).toContainText("Sets the target to −11.2 LUFS, the quietest reference, and keeps the −1 dBTP ceiling");
  await expect(page.getByRole("button", { name: "Destination" })).toHaveClass(/warn/);
  await expect(row(page, "Reference A").locator(".warnc")).toBeVisible();
  // 9s: the menu says where each destination sits.
  await page.getByRole("button", { name: "Destination" }).click();
  await expect(page.getByRole("menuitem", { name: /Streaming/ })).toContainText("2.8 LU quieter");
  await expect(page.getByRole("menuitem", { name: /^CD/ })).toContainText("Between your references");
  await expect(page.getByRole("menuitem", { name: /Match the references · −11.2 LUFS/ })).toBeVisible();
  await page.keyboard.press("Escape");
  // 9t: Keep, one undo step; the kept line offers to warn again.
  await warn.getByRole("button", { name: "Keep −18 LUFS" }).click();
  await expect(warn).toHaveCount(0);
  await expect(page.getByTestId("target-kept")).toContainText("6.8 LU quieter than Reference B · kept on purpose");
  await expect(page.getByRole("button", { name: "Destination" })).not.toHaveClass(/warn/);
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(warn).toBeVisible();
  await warn.getByRole("button", { name: "Keep −18 LUFS" }).click();
  await page.getByRole("button", { name: "Warn me again" }).click();
  await expect(warn).toBeVisible();
  // 10d: the same warning above the formats; Export stays available.
  await references(page, [{ id: "ref1", name: "learn-to-fly.wav", lufs: -9.6, true_peak_db: -0.3 }]);
  await page.getByRole("button", { name: "Export", exact: true }).click();
  const dlg = page.getByRole("dialog", { name: "Export master" });
  await expect(dlg.getByTestId("export-target-warning")).toContainText("This master will be 8.4 LU quieter than your reference");
  await dlg.getByRole("button", { name: /Export 1 file/ }).click();
  // 10e: the report says it in amber with the way back.
  const rep = page.getByRole("dialog", { name: "Export complete" });
  await expect(rep.getByTestId("report-level")).toContainText("quieter than Reference A.");
  await expect(rep.getByTestId("report-level")).toContainText("choose “Match the reference” in the Loudness card");
  await rep.getByRole("button", { name: "Done" }).click();
  // Match: the nearest reference, the ceiling kept, one undo step.
  await page.getByTestId("target-warning").getByRole("button", { name: "Match the reference" }).click();
  await expect(page.getByTestId("destination")).toHaveText("Custom · −9.6 LUFS · −1 dBTP");
  await expect(page.getByTestId("target-warning")).toHaveCount(0);
});

// req: MAS-2
test("target note: much louder than the reference, neutral and only in the Loudness card", async ({ page }) => {
  await toMaster(page);
  await references(page, [{ id: "ref1", name: "quiet.wav", lufs: -14.2 }]);
  await page.getByRole("button", { name: "Destination" }).click();
  await page.getByRole("menuitem", { name: /Loud · −9/ }).click();
  const note = page.getByTestId("target-warning");
  await expect(note).toHaveAttribute("role", "note");
  await expect(note).toContainText("At −9 LUFS this master will be 5.2 LU louder than Reference A (−14.2 LUFS). Getting there takes more limiting");
  await expect(page.getByRole("button", { name: "Destination" })).not.toHaveClass(/warn/);
  await page.getByRole("button", { name: "Export", exact: true }).click();
  await expect(page.getByTestId("export-target-warning")).toHaveCount(0);
});

// req: UI-7, MAS-7
test("the preparation overlay going away leaves the focus where the user put it (the export's fields keep what is typed)", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await simulatePrep(page, 300);
  // While the overlay fades out it lets clicks through: open Export and type before it is gone. The fade is too short
  // to be caught by polling from here, so the page itself clicks whatever is on top of the Export button at the very
  // moment the overlay starts leaving: the overlay, if it were still taking the pointer.
  await page.evaluate(() => {
    const w = window as unknown as { __leavingClick?: { onButton: boolean } };
    const seen = new MutationObserver(() => {
      if (!document.querySelector(".scrim.prep.is-leaving")) return;
      const btn = [...document.querySelectorAll("button")].find((b) => b.textContent?.trim() === "Export");
      if (!btn) return;
      seen.disconnect();
      const r = btn.getBoundingClientRect();
      const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2) as HTMLElement | null;
      w.__leavingClick = { onButton: !!top && btn.contains(top) };
      top?.click();
    });
    seen.observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ["class"] });
  });
  await bounceToMaster(page);
  await expect(page.getByTestId("prep-overlay")).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as unknown as { __leavingClick?: { onButton: boolean } }).__leavingClick?.onButton), { timeout: 10_000 }).toBe(true);
  const dlg = page.getByRole("dialog", { name: "Export master" });
  await dlg.getByLabel("Title").click();
  await expect(page.locator(".scrim.prep")).toHaveCount(0);
  await expect(dlg.getByLabel("Title")).toBeFocused();
  await page.keyboard.type("Confidence");
  await expect(dlg.getByLabel("Title")).toHaveValue("Confidence");
});

// req: AI-23, MAS-6
test("a master proposal that was discarded is reviewed again from Proposals and applied after a confirmation", async ({ page }) => {
  await toMaster(page);
  const assistant = page.getByRole("complementary", { name: "Assistant" });
  await assistant.getByRole("radio", { name: "Proposals" }).click();
  await expect(assistant).toContainText("No proposals yet");
  await assistant.getByRole("radio", { name: "Notes" }).click();
  await page.getByRole("button", { name: "Master with AI" }).click();
  const panel = page.getByRole("complementary", { name: "Master proposal" });
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await panel.getByRole("button", { name: "Discard" }).click();
  await expect(panel).toBeHidden();
  // It is not lost: it is in Proposals, with when it came and what became of it.
  await assistant.getByRole("radio", { name: "Proposals" }).click();
  const item = assistant.getByRole("list", { name: "Proposals received" }).getByRole("listitem");
  await expect(item).toHaveCount(1);
  await expect(item).toContainText("Master with AI");
  await expect(item).toContainText("Discarded");
  await item.getByRole("button", { name: /Review/ }).click();
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel.getByTestId("reopened-note")).toContainText("Received");
  await panel.getByRole("button", { name: "Apply all" }).click();
  const confirm = page.getByRole("dialog", { name: "Apply this proposal to your current master?" });
  await confirm.getByRole("button", { name: "Apply all" }).click();
  await expect(panel).toBeHidden();
  await assistant.getByRole("radio", { name: "Proposals" }).click();
  await expect(item).toHaveCount(1);
  await expect(item).toContainText("Applied");
  await assistant.getByRole("radio", { name: "History" }).click();
  await expect(assistant).toContainText("Master proposal applied");
});
