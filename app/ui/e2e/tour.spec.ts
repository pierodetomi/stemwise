import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { STEP_TEXT, TOUR_TEXT } from "../src/tour/text";

// The guided tour (P-020, design 18) on the mock backend: it starts by itself only when asked (`?tour`, as the core
// does on the very first launch), and its demo is the mock's own song.

type Mock = { __stemwiseMock: { tourShown: boolean; toursEnded: number; tourFails: boolean; tourBuildMs: number } };
const mock = <K extends keyof Mock["__stemwiseMock"]>(page: Page, k: K) =>
  page.evaluate((key) => (window as unknown as Mock).__stemwiseMock[key as K], k) as Promise<Mock["__stemwiseMock"][K]>;
const shot = (page: Page, name: string) => page.screenshot({ path: `test-results/visual/${name}.png` });

const tour = (page: Page) => page.getByTestId("tour");
const title = (page: Page) => tour(page).locator("#tt-title");

async function atStep(page: Page, n: number) {
  await expect(tour(page)).toHaveAttribute("data-step", String(n));
  await expect(title(page)).toHaveText(STEP_TEXT[n - 1].title);
}

/** Next, once the step is ready (a step that waits keeps it off). */
async function next(page: Page, n: number) {
  const b = tour(page).getByRole("button", { name: n === 1 ? TOUR_TEXT.start : TOUR_TEXT.next, exact: true });
  await expect(b).toBeEnabled({ timeout: 15_000 });
  await b.click();
}

/** The bubble of a lit step: beside a visible hole, inside the window. */
async function lit(page: Page) {
  const tip = tour(page).locator(".tour-tip");
  await expect(tip).toBeVisible();
  await expect(tour(page).locator(".tour-spot")).toBeVisible();
  const vp = page.viewportSize()!;
  await expect
    .poll(async () => {
      const b = await tip.boundingBox();
      return !!b && b.x >= 15 && b.y >= 15 && b.x + b.width <= vp.width - 15 && b.y + b.height <= vp.height - 15;
    })
    .toBe(true);
}

// req: UI-17, PRJ-9
test("first launch: the tour starts by itself on its opening card and marks itself shown", async ({ page }) => {
  await page.goto("/?tour");
  await atStep(page, 1);
  await expect(tour(page).getByRole("dialog")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Stemwise" })).toBeVisible();
  expect(await mock(page, "tourShown")).toBe(true);
  await shot(page, "18a-tour-opening");
  // The demo opens behind the card: the screen stays the Welcome until Start.
  await next(page, 1);
  await atStep(page, 2);
  await expect(page.getByLabel("Top bar")).toContainText(TOUR_TEXT.status);
});

// req: UI-17
test("no automatic start without the first-launch condition, and none again once shown", async ({ page }) => {
  await page.goto("/");
  await page.waitForTimeout(500);
  await expect(tour(page)).toHaveCount(0);
  await page.goto("/?tour");
  await atStep(page, 1);
  await page.keyboard.press("Escape");
  await expect(tour(page)).toHaveCount(0);
  const auto = await page.evaluate(() => (window as unknown as { __stemwiseMock: { tourInfo(): Promise<{ auto: boolean }> } }).__stemwiseMock.tourInfo().then((t) => t.auto));
  expect(auto).toBe(false);
});

// req: UI-18, UI-19, PRJ-9
test("every step forward and back, each with the state it declares, and Done back on the Welcome screen", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/?tour");
  await atStep(page, 1);
  for (let n = 1; n < 15; n++) {
    await next(page, n);
    await atStep(page, n + 1);
    if (n + 1 < 15) await lit(page);
    // What each step sets up by itself.
    if (n + 1 === 4) await expect(page.locator(".row.is-sel").first()).toContainText("Lead Vox");
    if (n + 1 === 8) await expect(page.getByLabel("Proposal", { exact: true })).toBeVisible({ timeout: 15_000 });
    if (n + 1 === 9) await expect(page.getByRole("list", { name: "History" })).toContainText(/Auto mix applied/);
    if (n + 1 === 10) await expect(page.getByLabel("Mix map")).toBeVisible();
    if (n + 1 === 12) await expect(page.getByLabel("Master chain").first()).toBeVisible({ timeout: 15_000 });
    if (n + 1 === 14) await expect(page.getByRole("dialog", { name: "Export master" })).toBeVisible();
    if ([2, 8, 10, 14].includes(n + 1)) await shot(page, `18-tour-step-${n + 1}`);
  }
  // Nothing is written in the tour: the export is off and says why.
  await page.keyboard.press("ArrowLeft");
  await atStep(page, 14);
  await expect(page.getByRole("dialog", { name: "Export master" }).getByRole("button", { name: /Export/ })).toBeDisabled();
  // Back, step by step: each step puts back what it declares.
  for (let n = 14; n > 1; n--) {
    await tour(page).getByRole("button", { name: TOUR_TEXT.back }).click();
    await atStep(page, n - 1);
    if (n - 1 === 13) await expect(page.getByRole("dialog", { name: "Export master" })).toHaveCount(0);
    if (n - 1 === 11) await expect(page.getByLabel("Timeline")).toBeVisible({ timeout: 15_000 });
    if (n - 1 === 9) await expect(page.getByLabel("Mix map")).toHaveCount(0);
    if (n - 1 === 8) await expect(page.getByLabel("Proposal", { exact: true })).toBeVisible({ timeout: 15_000 });
    if (n - 1 === 7) await expect(page.getByLabel("Proposal", { exact: true })).toHaveCount(0);
  }
  // And to the end again, with the keyboard.
  await page.evaluate(() => {
    const w = window as unknown as { __keys: string[] };
    w.__keys = [];
    const log = (phase: string) => (e: KeyboardEvent) => {
      const a = document.activeElement as HTMLElement | null;
      const t = e.target as HTMLElement;
      const b = document.querySelector('[data-testid="tour"] .btn.primary') as HTMLElement | null;
      w.__keys.push(`${phase} ${e.key} step=${document.querySelector('[data-testid="tour"]')?.getAttribute("data-step")} target=${t.tagName}.${t.className}[${t.getAttribute("role")}|${t.getAttribute("aria-label")}] active=${a?.tagName}.${a?.className}[${a?.getAttribute("role")}] prevented=${e.defaultPrevented} next(aria-disabled=${b?.getAttribute("aria-disabled")},disabled=${b?.hasAttribute("disabled")},text=${b?.textContent}) dialogs=${[...document.querySelectorAll('[role="dialog"],[role="alertdialog"]')].map((d) => d.getAttribute("aria-label")).join("/")}`);
    };
    document.addEventListener("keydown", log("bubble"), false);
  });
  for (let n = 1; n < 15; n++) {
    const b = tour(page).getByRole("button", { name: n === 1 ? TOUR_TEXT.start : TOUR_TEXT.next, exact: true });
    await expect(b).toBeEnabled({ timeout: 15_000 });
    await page.keyboard.press("ArrowRight");
    try {
      await atStep(page, n + 1);
    } catch (e) {
      console.log("STUCK at", n, JSON.stringify(await page.evaluate(() => (window as unknown as { __keys: string[] }).__keys.slice(-3)), null, 1));
      throw e;
    }
  }
  await shot(page, "18o-tour-closing");
  await tour(page).getByRole("button", { name: TOUR_TEXT.done }).click();
  await expect(tour(page)).toHaveCount(0);
  await expect(page.getByRole("button", { name: "New project" })).toBeVisible();
  expect(await mock(page, "toursEnded")).toBe(1);
  // Done says nothing more: the closing card already said where to find the tour again.
  await expect(page.locator(".toast").filter({ hasText: TOUR_TEXT.skippedTitle })).toHaveCount(0);
});

// req: UI-19
test("Skip tour and Esc end the tour the same way, with a toast that says where to find it", async ({ page }) => {
  await page.goto("/?tour");
  await next(page, 1);
  await next(page, 2);
  await atStep(page, 3);
  await tour(page).getByRole("button", { name: TOUR_TEXT.skip }).click();
  await expect(tour(page)).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Recent projects" })).toBeVisible();
  await expect(page.locator(".toast").filter({ hasText: TOUR_TEXT.skippedTitle })).toContainText(TOUR_TEXT.skippedText);
  await shot(page, "18v-tour-skipped");
  // Again from the Welcome link, then Esc in the middle.
  await page.getByRole("button", { name: /Take the tour/ }).click();
  await next(page, 1);
  await next(page, 2);
  await next(page, 3);
  await atStep(page, 4);
  await page.keyboard.press("Escape");
  await expect(tour(page)).toHaveCount(0);
  await expect(page.locator(".toast").filter({ hasText: TOUR_TEXT.skippedTitle }).first()).toBeVisible();
  expect(await mock(page, "toursEnded")).toBe(2);
});

// req: UI-18
test("while the tour runs, the app's shortcuts wait and clicks outside the lit area do nothing", async ({ page }) => {
  await page.goto("/?tour");
  await next(page, 1);
  await next(page, 2);
  await atStep(page, 3);
  // ⌘E would open Explain; ⌘I would hide the inspector.
  await page.keyboard.press("Meta+E");
  await page.keyboard.press("Meta+I");
  await expect(page.getByLabel("Mix map")).toHaveCount(0);
  await expect(page.getByLabel("Inspector", { exact: true })).toBeVisible();
  // A click on the assistant (outside the lit timeline) stops at the tour.
  const box = (await page.getByLabel("Assistant", { exact: true }).boundingBox())!;
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await atStep(page, 3);
  // Inside the lit area the app works: Space plays.
  await page.keyboard.press("Space");
  await expect(page.getByTestId("position")).not.toHaveText(/^0:00/);
});

// req: UI-17, UI-20
test("Help › Take the Tour from the app menu, asking first about unsaved changes", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
  // Not dirty: straight to the tour.
  await page.getByTestId("project-name").click();
  await page.getByRole("menuitem", { name: "Help" }).hover();
  await shot(page, "18t-tour-help-menu");
  await page.getByRole("menuitem", { name: TOUR_TEXT.menuItem }).click();
  await atStep(page, 1);
  await page.keyboard.press("Escape");
  // Dirty: the question of closing first, with why.
  await page.getByRole("button", { name: /Confidence/ }).click();
  await page.getByTestId("strip-Kick").locator(".fader").dblclick();
  await page.getByTestId("project-name").click();
  await page.getByRole("menuitem", { name: "Help" }).click();
  await page.getByRole("menuitem", { name: TOUR_TEXT.menuItem }).click();
  const ask = page.getByRole("dialog", { name: "Save changes to “Confidence”?" });
  await expect(ask).toContainText(TOUR_TEXT.saveFirstSub);
  await shot(page, "18u-tour-save-first");
  await ask.getByRole("button", { name: "Cancel" }).click();
  await expect(tour(page)).toHaveCount(0);
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await page.getByTestId("project-name").click();
  await page.getByRole("menuitem", { name: "Help" }).click();
  await page.getByRole("menuitem", { name: TOUR_TEXT.menuItem }).click();
  await page.getByRole("dialog", { name: /Save changes/ }).getByRole("button", { name: "Don’t save" }).click();
  await atStep(page, 1);
});

// req: UI-18, UI-4, UI-7
test("minimum window: every lit step keeps its bubble inside the window", async ({ page }) => {
  test.setTimeout(120_000);
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/?tour");
  for (let n = 1; n < 14; n++) {
    await next(page, n);
    await atStep(page, n + 1);
    await lit(page);
    if (n + 1 === 4) await shot(page, "18r-tour-min");
  }
});

// req: UI-7, UI-18
test("accessibility: the bubble is a labelled dialog, steps are announced, focus stays on the tour", async ({ page }) => {
  await page.goto("/?tour");
  const check = async (where: string) => {
    // After the bubble has faded in: mid-fade its text is not yet at full contrast.
    await page.waitForTimeout(400);
    const r = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
    const bad = r.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(bad.map((v) => `${where}: ${v.id} (${v.nodes.length}) ${v.nodes[0]?.target.join(" ")}`)).toEqual([]);
  };
  await atStep(page, 1);
  await expect(page.getByRole("dialog", { name: STEP_TEXT[0].title })).toBeVisible();
  await expect(tour(page).getByRole("button", { name: TOUR_TEXT.start })).toBeFocused();
  await check("opening card");
  await next(page, 1);
  await atStep(page, 2);
  await expect(page.getByRole("dialog", { name: STEP_TEXT[1].title })).toBeVisible();
  await expect(tour(page).getByRole("status").last()).toHaveText(TOUR_TEXT.announce(2, 15, STEP_TEXT[1].title));
  await expect(tour(page).getByRole("button", { name: TOUR_TEXT.next })).toBeFocused();
  await check("top bar step");
  // Tab goes round the bubble and the lit top bar, nothing else.
  for (let i = 0; i < 12; i++) {
    await page.keyboard.press("Tab");
    const inside = await page.evaluate(() => {
      const a = document.activeElement;
      return !!a && (!!a.closest(".tour-tip") || !!a.closest('header[aria-label="Top bar"]'));
    });
    expect(inside).toBe(true);
  }
});

// req: UI-19, PRJ-9
test("a demo that can't be prepared: the tour says so and ends, the app stays usable", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => ((window as unknown as Mock).__stemwiseMock.tourFails = true));
  await page.getByRole("button", { name: /Take the tour/ }).click();
  await expect(title(page)).toHaveText(TOUR_TEXT.failedTitle);
  await shot(page, "18w-tour-failed");
  await tour(page).getByRole("button", { name: TOUR_TEXT.close }).click();
  await expect(tour(page)).toHaveCount(0);
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
});

/** Straight to a step with Next. */
async function reach(page: Page, n: number) {
  await page.goto("/?tour");
  for (let i = 1; i < n; i++) await next(page, i);
  await atStep(page, n);
}

const appliedInHistory = (page: Page) =>
  page.evaluate(async () => {
    const m = (window as unknown as { __stemwiseMock: { history(p: string): Promise<[{ ai_call: string | null; undone: boolean }[], number]> } }).__stemwiseMock;
    return (await m.history("mix"))[0].some((e) => !!e.ai_call && !e.undone);
  });

// req: UI-18
test("step 8: applying or discarding the proposal in the lit area never holds the tour back", async ({ page }) => {
  await reach(page, 8);
  const proposal = page.getByLabel("Proposal", { exact: true });
  const nextB = tour(page).getByRole("button", { name: TOUR_TEXT.next, exact: true });
  await expect(proposal).toBeVisible({ timeout: 15_000 });
  // Apply all, as the step's own text invites to: Next stays there, nothing says "preparing".
  await proposal.getByRole("button", { name: "Apply all" }).click();
  await expect(proposal).toHaveCount(0);
  await expect(tour(page).locator(".tt-wait")).toHaveCount(0);
  await expect(nextB).toBeEnabled();
  expect(await appliedInHistory(page)).toBe(true);
  // Back on the step from the next one, it shows the proposal in review again, on the mix as it was.
  await next(page, 8);
  await atStep(page, 9);
  await tour(page).getByRole("button", { name: TOUR_TEXT.back, exact: true }).click();
  await atStep(page, 8);
  await expect(proposal).toBeVisible({ timeout: 15_000 });
  expect(await appliedInHistory(page)).toBe(false);
  // Discard asks for confirmation above the tour: both its buttons can be clicked.
  await proposal.getByRole("button", { name: "Discard" }).click();
  const confirm = page.locator(".scrim.over-tour .dialog");
  await expect(confirm).toBeVisible();
  for (const name of ["Cancel", "Discard"]) {
    const onTop = await confirm.getByRole("button", { name, exact: true }).evaluate((b) => {
      const r = b.getBoundingClientRect();
      return document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2) === b;
    });
    expect(onTop, name).toBe(true);
  }
  // Esc answers the confirmation, not the tour.
  await page.keyboard.press("Escape");
  await expect(confirm).toHaveCount(0);
  await atStep(page, 8);
  await proposal.getByRole("button", { name: "Discard" }).click();
  await confirm.getByRole("button", { name: "Discard", exact: true }).click();
  await expect(proposal).toHaveCount(0);
  await expect(tour(page).locator(".tt-wait")).toHaveCount(0);
  await expect(nextB).toBeEnabled();
  // The next step still shows the proposal applied. (→ and not a click: the bubble moves to the narrower lit area.)
  await page.keyboard.press("ArrowRight");
  await atStep(page, 9);
  await expect.poll(() => appliedInHistory(page), { timeout: 15_000 }).toBe(true);
});

// req: UI-18, PRJ-9
test("step 8: Cancel while the proposal is being prepared stops the waiting, and the tour goes on", async ({ page }) => {
  await reach(page, 8);
  const cancel = page.getByLabel("Assistant", { exact: true }).getByRole("button", { name: "Cancel", exact: true });
  await cancel.click();
  await expect(tour(page).locator(".tt-wait")).toHaveCount(0);
  await expect(page.locator(".toast")).toHaveCount(0);
  await next(page, 8);
  await atStep(page, 9);
  await expect.poll(() => appliedInHistory(page), { timeout: 15_000 }).toBe(true);
  await expect(tour(page).getByRole("button", { name: TOUR_TEXT.next, exact: true })).toBeEnabled({ timeout: 15_000 });
});

// req: UI-18, UI-7
test("keys on what has the focus: ↩ presses Back, Space plays from the bubble, ⌘ shortcuts wait in a field", async ({ page }) => {
  await reach(page, 3);
  // The focus is on Next: Space is the transport's, and the step does not change.
  await expect(tour(page).getByRole("button", { name: TOUR_TEXT.next, exact: true })).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.getByTestId("position")).not.toHaveText(/^0:00/);
  await atStep(page, 3);
  await page.keyboard.press("Space");
  // ↩ on Back goes back, not forward.
  await tour(page).getByRole("button", { name: TOUR_TEXT.back, exact: true }).focus();
  await page.keyboard.press("Enter");
  await atStep(page, 2);
  // ↩ on Next is still Next.
  await expect(tour(page).getByRole("button", { name: TOUR_TEXT.next, exact: true })).toBeFocused();
  await page.keyboard.press("Enter");
  await atStep(page, 3);
  for (let i = 3; i < 7; i++) await next(page, i);
  await atStep(page, 7);
  // From the request field of the lit assistant, ⌘, and ⌘E are still the app's and wait; typing works.
  const field = page.getByLabel("Assistant", { exact: true }).locator("textarea").first();
  await field.click();
  await page.keyboard.press("Meta+,");
  await page.keyboard.press("Meta+E");
  await page.keyboard.type("more air");
  await expect(field).toHaveValue(/more air/);
  await expect(page.getByLabel("Assistant", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Mix map")).toHaveCount(0);
  await expect(tour(page).locator(".tour-tip")).toBeVisible();
  await atStep(page, 7);
});
