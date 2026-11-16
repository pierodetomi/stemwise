import { expect, test, type Page } from "@playwright/test";

// The mix at a glance (F13, P-013) on the mock backend: chain marks on headers and strips (UI-8), every send named
// (UI-9), what the selection is connected to (UI-10), the mixer's Overview (UI-11).

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}
const header = (page: Page, name: string) => page.getByLabel(`${name} header`);
const strip = (page: Page, name: string) => page.getByTestId(`strip-${name}`);
const mock = (page: Page, f: string) => page.evaluate(`window.__stemwiseMock.${f}`);
const undoCount = async (page: Page) => {
  await page.getByRole("radio", { name: "History" }).click();
  const n = await page.getByRole("list", { name: "History" }).getByRole("listitem").count();
  await page.getByRole("radio", { name: "Notes" }).click();
  return n;
};

// req: UI-8
test("chain marks show on headers and strips without selecting anything, at every lane height", async ({ page }) => {
  await openDemo(page);
  const marks = (name: string) => page.getByTestId(`marks-${name}`);
  // Bass DI (16t): high-pass and EQ working, no low-pass, saturation off, compressor keyed by Kick, a bass
  // enhancer, hand-drawn automation.
  const bass = marks("Bass DI");
  await expect(bass.locator("[data-block=hpf]")).toHaveAttribute("data-state", "on");
  await expect(bass.locator("[data-block=lpf]")).toHaveCount(0);
  await expect(bass.locator("[data-block=sat]")).toHaveAttribute("data-state", "off");
  await expect(bass.locator("[data-block=bass]")).toHaveText("BAS");
  await expect(bass.locator("[data-block=comp]")).toHaveAttribute("aria-label", "Compressor · listens to Kick");
  await expect(bass.locator("[data-block=comp]")).toHaveClass(/key t2/);
  await expect(bass.getByRole("img", { name: /^Automation · Fader$/ })).toBeVisible();
  // Room: EQ on but flat reads neutral; Snare's compressor is parallel; Lead Vox's lanes are the assistant's.
  await expect(marks("Room").locator("[data-block=eq]")).toHaveAttribute("data-state", "neutral");
  await expect(marks("Overheads").locator("[data-block=lpf]")).toHaveAttribute("data-state", "off");
  // Snare's chain is six blocks long: four marks and "+2", whose tooltip names the parallel compressor (16t).
  await expect(marks("Snare").getByTestId("marks-more")).toHaveText("+2");
  await expect(marks("Snare").getByTestId("marks-more")).toHaveAttribute("title", /Compressor · parallel, 35 %/);
  await expect(marks("Lead Vox").locator(".pa.ai")).toHaveAttribute("title", /written by the assistant/);
  // A dot per return, lit where the track sends.
  await expect(marks("Lead Vox").getByRole("img", { name: "Plate · −14.0 dB" })).toBeVisible();
  await expect(marks("Kick").getByRole("img", { name: "Plate · no send" })).toBeVisible();
  // Groups carry the marks of their own chain.
  await expect(marks("Drums").locator("[data-block=comp]")).toHaveAttribute("aria-label", "Compressor · parallel, 50 %");
  // Strips: the same labels, one under the other.
  await expect(strip(page, "Bass DI").getByRole("button", { name: "Bass DI · Compressor · listens to Kick" })).toBeVisible();
  // Every lane height and the minimum window keep them.
  const body = page.locator("body");
  await body.press("Alt+Meta+-");
  await body.press("Alt+Meta+-");
  await expect(bass.locator("[data-block=comp]")).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 720 });
  await expect(bass.locator("[data-block=sat]")).toBeInViewport();
  const box = await bass.boundingBox();
  const name = await header(page, "Bass DI").locator(".nm").boundingBox();
  expect(box!.x).toBeGreaterThan(name!.x);
});

// req: UI-8, HIS-1
test("a click on a strip's label bypasses the block in one undo step; a double click opens it in the inspector", async ({ page }) => {
  await openDemo(page);
  const steps = await undoCount(page);
  const sat = strip(page, "Overheads").getByRole("button", { name: /^Overheads · EQ/ });
  await expect(sat).toHaveAttribute("data-state", "on");
  await sat.click();
  await expect(sat).toHaveAttribute("data-state", "off");
  // Header and inspector agree, and it is one history step.
  await expect(page.getByTestId("marks-Overheads").locator("[data-block=eq]")).toHaveAttribute("data-state", "off");
  expect(await undoCount(page)).toBe(steps + 1);
  await page.keyboard.press("ControlOrMeta+z");
  await expect(sat).toHaveAttribute("data-state", "on");
  // Keyboard: the label is a button.
  await sat.focus();
  await page.keyboard.press("Enter");
  await expect(sat).toHaveAttribute("data-state", "off");
  await page.keyboard.press("ControlOrMeta+z");
  // Double click: selects Kick and opens its compressor, without toggling anything.
  const comp = strip(page, "Kick").getByRole("button", { name: /^Kick · Compressor/ });
  await comp.dblclick();
  await expect(page.getByTestId("inspector-name")).toHaveText("Kick");
  await expect(page.getByTestId("row-comp").locator(".rname")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByLabel("Compressor editor")).toBeVisible();
  await page.waitForTimeout(400);
  await expect(comp).toHaveAttribute("data-state", "on");
  await page.getByRole("radio", { name: "History" }).click();
  await expect(page.getByRole("list", { name: "History" })).not.toContainText("Compressor");
});

// req: UI-9
test("every return has its send row in each strip, named on the rail, and a dot on each header", async ({ page }) => {
  await openDemo(page);
  await mock(page, "simulateReturns(4)");
  const rail = page.getByLabel("Mixer rows");
  for (const name of ["Plate", "Echo", "Slap", "Throw"]) await expect(rail.getByText(name, { exact: true })).toBeVisible();
  await expect(strip(page, "Lead Vox").getByRole("group", { name: "Sends" }).locator(":scope > div")).toHaveCount(4);
  await expect(page.getByTestId("marks-Lead Vox").locator(".sd")).toHaveCount(4);
  // The rows line up with the rail.
  const label = await rail.getByText("Throw", { exact: true }).boundingBox();
  const row = await strip(page, "Kick").getByRole("group", { name: "Sends" }).locator(":scope > div").nth(3).boundingBox();
  expect(Math.abs(label!.y + label!.height / 2 - (row!.y + row!.height / 2))).toBeLessThan(2);
  // A return lists its senders with colour, name and level, selected or not.
  const feeds = page.getByTestId("feeds-Plate");
  await feeds.scrollIntoViewIfNeeded();
  await expect(feeds).toContainText("4 tracks send here");
  await expect(feeds).toContainText("Snare−18.0");
  await expect(feeds).toContainText("BVs−12.0");
});

// req: UI-10
test("selecting a return marks who sends to it; selecting a track marks its group, returns and listeners", async ({ page }) => {
  await openDemo(page);
  await strip(page, "Plate").locator(".nm").click();
  const snareRow = page.locator(".row", { has: header(page, "Snare") });
  await expect(snareRow).toHaveClass(/is-rel/);
  await expect(header(page, "Snare").locator(".rl")).toHaveText("sends −18.0 dB");
  await expect(strip(page, "Snare")).toHaveClass(/is-rel/);
  await expect(strip(page, "Snare").locator(".hbar.rel")).toHaveCount(1);
  await expect(page.getByTestId("marks-Snare").locator(".sd.rel")).toHaveCount(1);
  await expect(strip(page, "Kick")).not.toHaveClass(/is-rel/);
  // Kick: Drums takes it, Bass DI listens to it.
  await header(page, "Kick").click();
  await expect(header(page, "Drums").locator(".rl")).toHaveText("takes Kick");
  await expect(header(page, "Bass DI").locator(".rl")).toHaveText("listens to Kick");
  await expect(strip(page, "Bass DI")).toHaveClass(/is-rel/);
  await expect(strip(page, "Snare")).not.toHaveClass(/is-rel/);
  // Snare: its return says why, with the level.
  await header(page, "Snare").click();
  await expect(header(page, "Plate").locator(".rl")).toHaveText("Snare −18.0 dB");
  // Selection never enters the history.
  await page.getByRole("radio", { name: "History" }).click();
  await expect(page.getByRole("list", { name: "History" })).not.toContainText("Snare");
});

// req: UI-11, UI-9
test("switching the mixer's view never moves the strips, and the row labels are whole in both", async ({ page }) => {
  await openDemo(page);
  const mixer = page.getByLabel("Mixer", { exact: true });
  const rail = page.getByLabel("Mixer rows");
  const layout = async () => ({
    rail: Math.round((await rail.boundingBox())!.width),
    strips: await mixer.locator(".strip:not(.mxrail)").evaluateAll((all) => all.slice(0, 6).map((s) => Math.round(s.getBoundingClientRect().left))),
    cut: await rail.locator(".lb, .cl").evaluateAll((all) => all.filter((l) => l.scrollWidth > l.clientWidth + 1).map((l) => l.textContent)),
  });
  const faders = await layout();
  expect(faders.cut, "row labels of the faders view are not cut").toEqual([]);
  await page.getByRole("radio", { name: "Overview" }).click();
  const overview = await layout();
  expect(overview.cut).toEqual([]);
  expect(overview.rail).toBe(faders.rail);
  expect(overview.strips).toEqual(faders.strips);
});

// req: UI-11, HIS-1
test("Overview: one cell per block, return, output and automation; a click bypasses in one step, an empty send is created", async ({ page }) => {
  await openDemo(page);
  const steps = await undoCount(page);
  await page.getByRole("radio", { name: "Overview" }).click();
  const bass = strip(page, "Bass DI");
  await expect(bass.locator(".cells")).toBeVisible();
  await expect(bass.getByRole("button", { name: /Compressor · listens to Kick · 4:1/ })).toBeVisible();
  await expect(strip(page, "Snare").getByRole("button", { name: /send to Plate · −18.0 dB/ })).toBeVisible();
  await expect(strip(page, "Plate").locator(".cells")).toContainText("4 in");
  await expect(bass.locator(".cells")).toContainText("Fader");
  const hp = bass.getByRole("button", { name: /^Bass DI · High-pass/ });
  await expect(hp).toHaveText("40");
  await hp.click();
  await expect(hp).toHaveText("off");
  await expect(page.getByTestId("marks-Bass DI").locator("[data-block=hpf]")).toHaveAttribute("data-state", "off");
  expect(await undoCount(page)).toBe(steps + 1);
  await page.keyboard.press("ControlOrMeta+z");
  await expect(hp).toHaveText("40");
  // An empty send cell creates the send at the default level.
  await bass.getByRole("button", { name: "Bass DI · add a send to Echo" }).click();
  await expect(bass.getByRole("button", { name: /send to Echo · −12.0 dB/ })).toBeVisible();
  // The view is view state: switching back keeps the history as it was.
  const after = await undoCount(page);
  await page.getByRole("radio", { name: "Faders" }).click();
  await expect(bass.getByRole("slider").last()).toBeVisible();
  expect(await undoCount(page)).toBe(after);
});

// req: NFR-3, UI-8, UI-9
test("with the marks on 32 headers and strips and 4 returns, playback keeps 60 fps with the mixer open @perf", async ({ page }) => {
  await page.setViewportSize({ width: 1920, height: 1200 });
  await openDemo(page);
  await mock(page, "simulateTracks(32)");
  await mock(page, "simulateReturns(4)");
  await expect(page.locator(".mixer .strip button.sg")).not.toHaveCount(0);
  await page.getByRole("button", { name: "Play" }).click();
  await page.waitForTimeout(500);
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
  console.log(`NFR-3 with marks and mixer: ${(1000 / mean).toFixed(1)} fps mean, ${slow} of ${times.length} frames over 33 ms`);
  expect(1000 / mean).toBeGreaterThanOrEqual(55);
  expect(slow / times.length).toBeLessThanOrEqual(0.05);
});
