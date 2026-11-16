import { expect, test, type Page } from "@playwright/test";

// The editors of the new blocks (F16, P-018; design 16d–16s) on the mock backend: each says what it does, its controls
// are parameters (automatable), its settings are one undo step.

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}
const header = (page: Page, name: string) => page.getByLabel(`${name} header`);
const insp = (page: Page) => page.getByLabel("Inspector", { exact: true });
const shot = (page: Page, name: string) => page.screenshot({ path: `test-results/visual/${name}.png` });
const open = async (page: Page, track: string, row: string) => {
  await header(page, track).click();
  await insp(page).getByTestId(`row-${row}`).locator(".rname").click();
};

// req: FX-13, FX-14, FX-15, FX-16, UI-12
test("dynamics editors: gate curve, de-esser modes, dynamic EQ keyed by the vocal, transient shape", async ({ page }) => {
  await openDemo(page);
  await open(page, "Kick", "gate");
  await expect(page.getByRole("img", { name: /Gate curve/ })).toBeVisible();
  await expect(insp(page).getByRole("slider", { name: "Kick · Range" })).toBeVisible();
  await shot(page, "16d-gate");
  await open(page, "Lead Vox", "deess");
  const mode = insp(page).getByRole("radiogroup", { name: "Mode" });
  await mode.getByRole("radio", { name: "Wide" }).click();
  await expect(mode.getByRole("radio", { name: "Wide" })).toHaveAttribute("aria-checked", "true");
  await expect(insp(page).getByTestId("row-deess")).toContainText("De-esser");
  await page.keyboard.press("ControlOrMeta+z");
  await expect(mode.getByRole("radio", { name: "Split" })).toHaveAttribute("aria-checked", "true");
  await open(page, "Gtr Left", "deq");
  await expect(insp(page).getByRole("button", { name: /Sidechain of Dynamic EQ: Lead Vox/ })).toBeVisible();
  await shot(page, "16e-dynamic-eq");
  await open(page, "Kick", "trans");
  await expect(page.getByRole("img", { name: "The hit as recorded and after" })).toBeVisible();
  await shot(page, "16f-transient");
});

// req: FX-18, FX-19, FX-20, FX-21, UI-12
test("colour, space, movement and pitch editors", async ({ page }) => {
  await openDemo(page);
  await open(page, "Lead Vox", "sat");
  const ch = insp(page).getByRole("radiogroup", { name: "Character" });
  await expect(ch.getByRole("radio", { name: "Tube" })).toHaveAttribute("aria-checked", "true");
  await ch.getByRole("radio", { name: "Tape" }).click();
  await expect(insp(page).getByTestId("row-sat")).toContainText("Tape");
  await shot(page, "16g-saturation");
  await open(page, "Bass DI", "bass");
  await expect(page.getByRole("img", { name: /Bass enhancer: harmonics/ })).toBeVisible();
  await shot(page, "16h-bass");
  await open(page, "Rhodes", "width");
  await expect(insp(page).getByRole("slider", { name: "Rhodes · Width" })).toBeVisible();
  await open(page, "Rhodes", "mod");
  const m = insp(page).getByRole("radiogroup", { name: "Mode" });
  await m.getByRole("radio", { name: "Phaser" }).click();
  await expect(insp(page).getByRole("radiogroup", { name: "Stages" })).toBeVisible();
  await shot(page, "16k-modulation");
  await open(page, "Gtr Right", "trem");
  await insp(page).getByRole("radiogroup", { name: "Mode" }).getByRole("radio", { name: "Auto-pan" }).click();
  await expect(insp(page).getByTestId("row-trem")).toContainText("Auto-pan");
  await shot(page, "16l-tremolo");
  // Pitch: a harmonizer added to the strings, two voices on the scale.
  await header(page, "Strings").click();
  await insp(page).getByTestId("add-block").click();
  await page.getByTestId("add-block-menu").getByRole("button", { name: /^Harmonizer:/ }).click();
  await insp(page).getByRole("radiogroup", { name: "Voices" }).getByRole("radio", { name: "2 voices" }).click();
  await expect(insp(page).getByRole("slider", { name: "Strings · Interval" })).toHaveCount(2);
  await expect(insp(page)).toContainText("do not follow the key");
  await shot(page, "16m-harmonizer");
  await open(page, "Lead Vox", "dbl");
  await expect(page.getByRole("img", { name: "The take and the double" })).toBeVisible();
  await shot(page, "16n-doubler");
});

// req: FX-22, FX-23, UI-12
test("returns: a shimmer reverb with its interval, a ping-pong tape delay", async ({ page }) => {
  await openDemo(page);
  await page.getByTestId("strip-Plate").locator(".nm").click();
  await insp(page).getByRole("radiogroup", { name: "Algorithm" }).getByRole("radio", { name: "Shimmer" }).click();
  const iv = insp(page).getByRole("radiogroup", { name: "Shimmer interval" });
  await iv.getByRole("radio", { name: "+7" }).click();
  await expect(iv.getByRole("radio", { name: "+7" })).toHaveAttribute("aria-checked", "true");
  await expect(insp(page).getByRole("slider", { name: "Plate · Shimmer" })).toBeVisible();
  await shot(page, "16s-shimmer");
  await page.getByTestId("strip-Echo").locator(".nm").click();
  await insp(page).getByRole("radiogroup", { name: "Delay mode" }).getByRole("radio", { name: "Ping-pong" }).click();
  await insp(page).getByRole("radiogroup", { name: "Delay character" }).getByRole("radio", { name: "Tape" }).click();
  await expect(insp(page).getByRole("slider", { name: "Echo · Wow & flutter" })).toBeVisible();
  await shot(page, "16r-delay");
  await page.keyboard.press("ControlOrMeta+z");
  await expect(insp(page).getByRole("slider", { name: "Echo · Wow & flutter" })).toHaveCount(0);
});
