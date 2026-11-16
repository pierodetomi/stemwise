import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { SHORTCUTS } from "../src/shared/shortcuts";

// The finishing pass (F9 gate) on the mock backend: shortcuts from one table (UI-3), Settings (SET-1), errors and
// empty states that say what happened and offer the way out (UI-6, design 13), basic accessibility (UI-7).

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}
type Mock = {
  simulateTracks(n: number): void;
  simulateMissing(n: number): void;
  simulateDisconnect(d: boolean): void;
  simulateNetworkDown(d: boolean): void;
  simulateEmpty(): void;
};
const mock = (page: Page, f: (m: Mock) => void) => page.evaluate(`(${f.toString()})(window.__stemwiseMock)`);
const shot = (page: Page, name: string) => page.screenshot({ path: `test-results/visual/${name}.png` });

// req: UI-3
test("shortcuts drive tools, view and transport, and Settings lists the same table", async ({ page }) => {
  await openDemo(page);
  const body = page.locator("body");
  await body.press("c");
  await expect(page.getByRole("radio", { name: "Split" })).toHaveAttribute("aria-checked", "true");
  await body.press("v");
  await expect(page.getByRole("radio", { name: "Select" })).toHaveAttribute("aria-checked", "true");
  const snap = page.getByRole("button", { name: /Snap/ });
  const before = await snap.getAttribute("aria-pressed");
  await body.press("n");
  await expect(snap).not.toHaveAttribute("aria-pressed", before ?? "");
  // S solos the selected track; ⇧M mutes it.
  await page.getByLabel("Bass DI header").click();
  await body.press("s");
  await expect(page.getByLabel("Bass DI header").getByRole("button", { name: "Solo" })).toHaveAttribute("aria-pressed", "true");
  await body.press("Shift+m");
  await expect(page.getByLabel("Bass DI header").getByRole("button", { name: "Mute" })).toHaveAttribute("aria-pressed", "true");
  // ⌘M hides the mixer; ⌘, opens Settings.
  await body.press("Meta+m");
  await expect(page.getByRole("button", { name: "Show mixer" })).toBeVisible();
  await body.press("Meta+,");
  await expect(page.getByRole("navigation", { name: "Settings sections" })).toBeVisible();
  await page.getByRole("button", { name: "Shortcuts" }).click();
  const list = page.getByTestId("shortcuts");
  for (const s of SHORTCUTS) await expect(list).toContainText(s.label);
  await page.getByLabel("Filter shortcuts").fill("loop");
  await expect(list.locator(".kbd")).toHaveCount(SHORTCUTS.filter((s) => /loop/i.test(s.label)).length);
  await shot(page, "11d-shortcuts");
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
});

// req: SET-1
test("settings: audio, assistant and appearance, kept after a restart", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Settings (⌘,)" }).click();
  // Audio (11a): device list and buffer.
  await page.getByRole("button", { name: "Output device" }).click();
  await expect(page.getByRole("menuitem", { name: /Scarlett 2i2/ })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("radiogroup", { name: "Buffer" }).getByRole("radio", { name: "256" }).click();
  await expect(page.getByText("256 samples · 5.3 ms at 48 kHz")).toBeVisible();
  await expect(page.getByRole("meter", { name: "Engine load" })).toBeVisible();
  await shot(page, "11a-audio");
  // AI (11b): decider, model, effort, totals; the key is never shown.
  await page.getByRole("button", { name: "AI" }).click();
  await page.getByRole("radiogroup", { name: "Decider" }).getByRole("radio", { name: "Offline demo" }).click();
  await expect(page.getByRole("radiogroup", { name: "Decider" }).getByRole("radio", { name: "Offline demo" })).toHaveAttribute("aria-checked", "true");
  await page.getByRole("radiogroup", { name: "Effort" }).getByRole("radio", { name: "High" }).click();
  await expect(page.getByRole("radiogroup", { name: "Effort" }).getByRole("radio", { name: "High" })).toHaveAttribute("aria-checked", "true");
  await expect(page.getByTestId("key-state")).toHaveText("Stored in the keychain");
  await expect(page.getByTestId("total-cost")).toHaveText("$0.48");
  await shot(page, "11b-ai");
  // Appearance (11c): waveforms off reaches the timeline and survives a reload.
  await page.getByRole("button", { name: "Appearance" }).click();
  await page.getByRole("switch", { name: "Waveforms" }).click();
  await shot(page, "11c-appearance");
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.locator(".clip .wf")).toHaveCount(0);
  await page.reload();
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByRole("button", { name: "Appearance" }).click();
  await expect(page.getByRole("switch", { name: "Waveforms" })).toHaveAttribute("aria-checked", "false");
  await page.getByRole("switch", { name: "Waveforms" }).click();
});

// req: UI-6
test("missing files are hatched, silent and located again", async ({ page }) => {
  await openDemo(page);
  await mock(page, (m) => m.simulateMissing(2));
  const banner = page.getByTestId("missing-banner");
  await expect(banner).toContainText("2 audio files are missing from the project folder");
  expect(await page.locator(".clip.is-missing").count()).toBeGreaterThan(0);
  await shot(page, "13d-missing");
  await banner.getByRole("button", { name: "Locate…" }).click();
  await page.getByRole("menuitem").first().click();
  await expect(banner).toContainText("is missing from the project folder");
  await banner.getByRole("button", { name: "Locate…" }).click();
  await expect(banner).toBeHidden();
  await expect(page.locator(".clip.is-missing")).toHaveCount(0);
});

// req: UI-6
test("a disconnected audio device is reported under the top bar with a way out", async ({ page }) => {
  await openDemo(page);
  await mock(page, (m) => m.simulateDisconnect(true));
  const banner = page.getByTestId("device-banner");
  await expect(banner).toContainText("Demo output was disconnected");
  await expect(banner).toContainText("Nothing in the project changed");
  await banner.getByRole("button", { name: /^Use / }).click();
  await expect(banner).toBeHidden({ timeout: 3000 });
});

// req: UI-6, AI-9
test("no network: the assistant says so in place and offers the offline demo", async ({ page }) => {
  await openDemo(page);
  await mock(page, (m) => m.simulateNetworkDown(true));
  await page.getByRole("button", { name: "Auto mix" }).click();
  const banner = page.getByTestId("offline-banner");
  await expect(banner).toContainText("No connection to OpenRouter");
  await expect(banner).toContainText("Your mix is not affected");
  await shot(page, "13f-offline");
  // The error toast sits over the foot of the column: dismissed first, as a user would.
  await page.getByRole("alert").filter({ hasText: "could not answer" }).getByRole("button", { name: "Dismiss" }).click();
  await banner.getByRole("button", { name: "Use offline demo" }).click();
  await expect(banner).toBeHidden();
});

// req: UI-6
test("an empty project says what each panel is for", async ({ page }) => {
  await openDemo(page);
  await mock(page, (m) => m.simulateEmpty());
  await expect(page.getByTestId("empty-timeline")).toContainText("Drop your stems here");
  await expect(page.getByTestId("ruler")).toBeHidden();
  await expect(page.getByRole("region", { name: "Mixer" })).toContainText("Strips appear here as you add tracks.");
  await expect(page.getByLabel("Inspector", { exact: true })).toContainText("Nothing selected");
  await expect(page.getByTestId("assistant-empty")).toContainText("Add tracks first");
  await expect(page.getByRole("button", { name: "Auto mix" })).toBeDisabled();
  await expect(page.getByText("No tracks · rate not set")).toBeVisible();
  await shot(page, "13a-empty");
  await page.getByRole("button", { name: "Choose files…" }).click();
  await expect(page.getByRole("button", { name: "Add tracks" })).toBeVisible();
});

// req: UI-7
test("accessibility: AA contrast, names on controls, keyboard on continuous controls", async ({ page }) => {
  const check = async (where: string) => {
    const r = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
    const bad = r.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(bad.map((v) => `${where}: ${v.id} (${v.nodes.length}) ${v.nodes[0]?.target.join(" ")}`)).toEqual([]);
  };
  await page.goto("/");
  await check("welcome");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await check("mix");
  await page.getByRole("button", { name: "Settings (⌘,)" }).click();
  for (const tab of ["Audio", "AI", "Appearance", "Shortcuts"]) {
    await page.getByRole("button", { name: tab, exact: true }).click();
    await check(`settings ${tab}`);
  }
  await page.getByRole("button", { name: "Done" }).click();
  // A clip takes the keyboard too: Tab to it, Enter selects it.
  const clip = page.locator(".clip").first();
  await clip.focus();
  await page.keyboard.press("Enter");
  await expect(clip).toHaveAttribute("aria-pressed", "true");
  // The proposal in review (4a) and AI usage.
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(page.getByRole("complementary", { name: "Proposal" })).toBeVisible({ timeout: 10_000 });
  await check("proposal");
  await page.getByRole("complementary", { name: "Proposal" }).getByRole("button", { name: /Discard/ }).click();
  await page.getByRole("dialog", { name: "Discard this proposal?" }).getByRole("button", { name: "Discard" }).click();
  // AI usage and the master workspace.
  await page.getByRole("button", { name: "AI usage" }).first().click();
  await check("AI usage");
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  // The preparation overlay (9d), slowed so it is checked still, then the master once the overlay is gone (a check
  // during its fade would read colours halfway).
  await page.evaluate(() => (window as unknown as { __stemwiseMock: { simulatePrep(ms: number): void } }).__stemwiseMock.simulatePrep(1500));
  await page.getByRole("radio", { name: "Master" }).click();
  await page.getByRole("dialog", { name: "Bounce & go to Master" }).getByRole("button", { name: "Bounce & go to Master" }).click();
  await page.getByRole("button", { name: "Open Master" }).click({ timeout: 10_000 });
  await expect(page.getByTestId("prep-overlay")).toBeVisible();
  await check("master preparing");
  await expect(page.getByLabel("Master chain")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("prep-overlay")).toHaveCount(0);
  await check("master");
  await page.getByRole("button", { name: "Export" }).click();
  await expect(page.getByRole("dialog", { name: "Export master" })).toBeVisible();
  await check("export");
  await page.getByRole("dialog", { name: "Export master" }).getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("radiogroup", { name: "Phase" }).getByRole("radio", { name: "Mix" }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
  // A fader takes the keyboard: focus, arrow up, the value moves.
  const fader = page.getByTestId("strip-Bass DI").getByRole("slider").last();
  await fader.focus();
  const v0 = Number(await fader.getAttribute("aria-valuenow"));
  await page.keyboard.press("ArrowUp");
  await expect.poll(async () => Number(await fader.getAttribute("aria-valuenow"))).toBeGreaterThan(v0);
});

// req: NFR-3
test("the timeline keeps 60 fps while playing with 32 visible tracks @perf", async ({ page }) => {
  await page.setViewportSize({ width: 1920, height: 1200 });
  await openDemo(page);
  await mock(page, (m) => m.simulateTracks(32));
  const body = page.locator("body");
  await body.press("Meta+m"); // mixer away: the rows get the height
  await body.press("Alt+Meta+-");
  const visible = await page.evaluate(() => {
    const box = document.querySelector(".tl-rows")!.getBoundingClientRect();
    return [...document.querySelectorAll(".tl-rows .row .clip")].filter((c) => {
      const r = c.getBoundingClientRect();
      return r.bottom > box.top && r.top < box.bottom;
    }).length;
  });
  const rows = await page.evaluate(() => {
    const box = document.querySelector(".tl-rows")!.getBoundingClientRect();
    return [...document.querySelectorAll(".tl-rows .row")].filter((r) => r.getBoundingClientRect().bottom <= box.bottom + 1).length;
  });
  expect(rows, "rows fully visible").toBeGreaterThanOrEqual(32);
  expect(visible).toBeGreaterThan(32);
  await page.getByRole("button", { name: "Play" }).click();
  await page.waitForTimeout(500);
  // Frame times over 3 s of playback (playhead, meters, follow).
  const times: number[] = await page.evaluate(
    () =>
      new Promise((done) => {
        const t: number[] = [];
        let last = performance.now();
        const step = (now: number) => {
          t.push(now - last);
          last = now;
          if (t.length < 180) requestAnimationFrame(step);
          else done(t);
        };
        requestAnimationFrame(step);
      }),
  );
  const mean = times.reduce((a, b) => a + b, 0) / times.length;
  const slow = times.filter((x) => x > 1000 / 30).length;
  console.log(`NFR-3: ${(1000 / mean).toFixed(1)} fps mean, ${slow} of ${times.length} frames over 33 ms`);
  expect(1000 / mean).toBeGreaterThanOrEqual(55);
  expect(slow / times.length).toBeLessThanOrEqual(0.05);
});

// req: UI-4, UI-3
test("one switch per panel in the top bar: each hides and shows its own panel, alone", async ({ page }) => {
  await openDemo(page);
  const sw = (name: string) => page.getByRole("group", { name: "Panels" }).getByRole("button", { name: `${name} panel` });
  const mixer = page.getByRole("region", { name: "Mixer" });
  const inspector = page.getByLabel("Inspector", { exact: true });
  const assistant = page.getByRole("complementary", { name: "Assistant" });
  const pressed = async () => Promise.all(["Mixer", "Inspector", "Assistant"].map((n) => sw(n).getAttribute("aria-pressed")));
  expect(await pressed()).toEqual(["true", "true", "true"]);
  // Each on its own: the other two stay as they are.
  await sw("Mixer").click();
  await expect(mixer).toBeHidden();
  await expect(inspector).toBeVisible();
  await expect(assistant).toBeVisible();
  expect(await pressed()).toEqual(["false", "true", "true"]);
  await sw("Assistant").click();
  await expect(assistant).toBeHidden();
  await expect(inspector).toBeVisible();
  await sw("Inspector").click();
  await expect(inspector).toBeHidden();
  expect(await pressed()).toEqual(["false", "false", "false"]);
  await expect(page.getByLabel("Timeline")).toBeVisible();
  // The same switch brings a panel back, and it follows the shortcut and the collapsed rail (one state).
  await sw("Inspector").click();
  await expect(inspector).toBeVisible();
  await expect(mixer).toBeHidden();
  await page.locator("body").press("Meta+m");
  await expect(mixer).toBeVisible();
  await expect(sw("Mixer")).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Show Assistant" }).click();
  await expect(sw("Assistant")).toHaveAttribute("aria-pressed", "true");
  await expect(sw("Mixer")).toHaveAttribute("title", "Hide mixer (⌘M)");
  // Minimum window with unsaved changes: the bar still fits, the saved state gives way (UI-4).
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByLabel("Kick header").click();
  await page.locator("body").press("Shift+l");
  await expect(page.getByLabel("Top bar")).toContainText("Unsaved changes");
  expect(await page.getByLabel("Top bar").evaluate((e) => e.scrollWidth <= e.clientWidth)).toBe(true);
  await expect(page.getByTestId("bus-meters")).toBeInViewport({ ratio: 1 });
});
