import { expect, test, type Page } from "@playwright/test";

// Screens of the design at the reference and minimum sizes, saved for the visual comparison with design/screens
// (FOUNDATION §8.1, F6 gate). The checks here are structural: every region is on screen and nothing overflows.
const shot = (page: Page, name: string) => page.screenshot({ path: `test-results/visual/${name}.png` });

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}

// req: UI-2
test("1 · welcome", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Recent projects" })).toBeVisible();
  await shot(page, "01-welcome");
});

// req: UI-2, UI-1
test("3a · mix workspace", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Play" }).click();
  await page.waitForTimeout(600);
  for (const region of ["Top bar", "Timeline", "Mixer", "Inspector", "Assistant"]) {
    await expect(page.getByLabel(region, { exact: true })).toBeVisible();
  }
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
  expect(overflow).toBe(false);
  await shot(page, "03a-mix");
});

// req: UI-4
test("3d · minimum window", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await openDemo(page);
  await expect(page.getByLabel("Inspector", { exact: true })).toBeVisible();
  await shot(page, "03d-min");
});

// req: UI-2
test("2c · import with files listed", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New project" }).click();
  await page.getByRole("button", { name: "Choose files…" }).click();
  await page.getByLabel("Project name").fill("Paper Lanterns");
  await expect(page.getByRole("table", { name: "Files to import" })).toBeVisible();
  await shot(page, "02c-import");
});

// req: UI-2, UI-11
test("3e · Overview in the mixer", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("radio", { name: "Overview" }).click();
  await expect(page.getByTestId("strip-Bass DI").locator(".cells")).toBeVisible();
  await shot(page, "03e-overview");
});

// req: UI-2, UI-8, UI-9
test("3f · a large project with small lanes, and 3g its Overview", async ({ page }) => {
  await openDemo(page);
  await page.evaluate("window.__stemwiseMock.simulateTracks(27); window.__stemwiseMock.simulateReturns(4)");
  const body = page.locator("body");
  await body.press("Alt+Meta+-");
  await body.press("Alt+Meta+-");
  await expect(page.getByLabel("Mixer rows")).toContainText("Throw");
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
  expect(overflow).toBe(false);
  await shot(page, "03f-large");
  await page.getByRole("radio", { name: "Overview" }).click();
  await shot(page, "03g-large-overview");
});

// req: UI-2, UI-10
test("7f · what feeds a return, 7g · what a track goes to, 7h · who listens to it", async ({ page }) => {
  await openDemo(page);
  await page.getByTestId("strip-Plate").locator(".nm").click();
  await expect(page.getByTestId("strip-Snare")).toHaveClass(/is-rel/);
  await shot(page, "07f-feeds-return");
  await page.getByLabel("Snare header").click();
  await shot(page, "07g-track-goes-to");
  await page.getByLabel("Kick header").click();
  await shot(page, "07h-listens");
});

// req: UI-2, HIS-1
test("3b · history", async ({ page }) => {
  await openDemo(page);
  await page.getByLabel("Lead Vox header").click();
  for (const name of ["Snare", "Room"]) await page.getByTestId(`strip-${name}`).locator(".fader").dblclick();
  await page.getByRole("radio", { name: "History" }).click();
  await shot(page, "03b-history");
});

// req: UI-2, TRN-3
test("6 · editing, tall lanes", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Track height" }).click();
  await page.getByRole("button", { name: /Clip Gtr Left/ }).nth(1).click();
  await page.getByRole("button", { name: /Clip Gtr Right/ }).nth(1).click({ modifiers: ["Shift"] });
  await shot(page, "06-editing");
});

// req: UI-2, FX-5
test("7a · reverb return", async ({ page }) => {
  await openDemo(page);
  await page.getByTestId("strip-Plate").locator(".nm").click();
  await shot(page, "07a-return");
});

// req: UI-2, TRK-3
test("7c · output menu", async ({ page }) => {
  await openDemo(page);
  await page.getByLabel("Lead Vox header").click();
  await page.getByRole("button", { name: /Output of Lead Vox/ }).click();
  await shot(page, "07c-output");
});

// req: UI-2, AI-12
test("5a · AI at work", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Play" }).click();
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(page.getByText("Auto mix in progress")).toBeVisible();
  await page.waitForTimeout(500);
  await shot(page, "05a-working");
});

// req: UI-2, AI-2
test("4a · proposal in review", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Auto mix" }).click();
  const panel = page.getByRole("complementary", { name: "Proposal" });
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await panel.getByRole("radio", { name: "Proposal" }).click();
  await shot(page, "04a-proposal");
  for (const n of ["Room", "Strings"]) await panel.getByTestId(`card-${n}`).getByRole("checkbox").click();
  await panel.getByRole("radio", { name: "Current mix" }).click();
  await shot(page, "04b-partial");
});

// req: UI-2, HIS-4
test("3c · snapshots", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("radio", { name: "Snapshots" }).click();
  for (const n of ["Rough balance", "Before auto mix"]) {
    await page.getByRole("button", { name: "Save snapshot" }).click();
    await page.getByLabel("Snapshot name").fill(n);
    await page.getByLabel("Snapshot name").press("Enter");
  }
  await shot(page, "03c-snapshots");
});

// req: UI-2, AI-14
test("12 · AI usage", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "AI usage" }).click();
  await shot(page, "12a-usage");
  await page.getByRole("table", { name: "AI calls" }).getByRole("row").nth(3).click();
  await shot(page, "12b-detail");
  await page.getByLabel("Call detail").getByRole("radio", { name: "Raw JSON" }).click();
  await shot(page, "12c-raw");
});

// req: UI-2, AI-16
test("12d · empty and 12e · spending cap", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New project" }).click();
  await page.getByRole("button", { name: "Choose files…" }).click();
  await page.getByLabel("Project name").fill("Empty usage");
  await page.getByRole("button", { name: "Create project" }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("button", { name: "Create project" }).click();
  await page.getByRole("button", { name: "AI usage" }).click();
  await shot(page, "12d-empty");
  await openDemo(page);
  await page.getByRole("button", { name: "AI usage" }).click();
  await page.getByLabel("Spending cap in USD").fill("0.50");
  await page.getByRole("button", { name: "Set cap" }).click();
  await page.getByRole("button", { name: "Back to Mix" }).click();
  await page.getByRole("button", { name: "Auto mix" }).click();
  await shot(page, "12e-cap");
});

// req: UI-2, MDN-1, MAS-4
test("8 · bounce and 9 · master workspace", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("radio", { name: "Master" }).click();
  await shot(page, "08a-bounce");
  await page.getByRole("dialog", { name: "Bounce & go to Master" }).getByRole("button", { name: "Bounce & go to Master" }).click();
  await expect(page.getByRole("dialog", { name: /Mixdown \d+ is ready/ })).toBeVisible({ timeout: 10_000 });
  await shot(page, "08c-result");
  await page.getByRole("button", { name: "Open Master" }).click();
  await page.getByRole("button", { name: "Play" }).click();
  await page.waitForTimeout(700);
  await shot(page, "09a-master");
  // 15a, 15c, 15d, 15e: the rack with the clipper, the compressor, a five-band multiband and a section's offsets.
  const rack = page.getByLabel("Master chain");
  await rack.getByRole("button", { name: "Clipper", exact: true }).click();
  await page.getByRole("region", { name: "Clipper" }).getByRole("button", { name: "Clipper on" }).click();
  await page.waitForTimeout(400);
  await shot(page, "15a-rack-clipper");
  await rack.getByRole("button", { name: "Compressor", exact: true }).click();
  await page.getByRole("region", { name: "Compressor" }).getByRole("button", { name: "Compressor on" }).click();
  await shot(page, "15c-compressor");
  await rack.getByRole("button", { name: "Multiband", exact: true }).click();
  const mb = page.getByRole("region", { name: "Multiband" });
  await mb.getByRole("button", { name: "Multiband on" }).click();
  await mb.getByRole("button", { name: "One band more" }).click();
  await mb.getByRole("button", { name: "One band more" }).click();
  await page.waitForTimeout(400);
  await shot(page, "15h-multiband-five");
  await page.getByTestId("add-block").click();
  await shot(page, "15i-add-block");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Offsets of Chorus" }).first().click();
  await shot(page, "15e-section");
  await page.getByTestId("section-editor").getByRole("button", { name: "Done" }).click();
  await page.getByRole("button", { name: "Master with AI" }).click();
  await expect(page.getByRole("complementary", { name: "Master proposal" })).toBeVisible({ timeout: 10_000 });
  await shot(page, "09c-proposal");
  await page.getByRole("button", { name: "Export" }).click();
  await shot(page, "10a-export");
  await page.getByRole("dialog", { name: "Export master" }).getByRole("button", { name: /Export \d files?/ }).click();
  await expect(page.getByRole("dialog", { name: "Export complete" })).toBeVisible({ timeout: 10_000 });
  await shot(page, "10c-report");
});
