import { expect, test, type Page } from "@playwright/test";

// The chain of blocks (F15, P-018) on the mock backend: the list in the inspector (16a), the "Add block" menu by family
// (16b), reordering (16c), the chain of a return (16p), marks and Overview with long chains (16t, 16u).

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}
const header = (page: Page, name: string) => page.getByLabel(`${name} header`);
const strip = (page: Page, name: string) => page.getByTestId(`strip-${name}`);
const insp = (page: Page) => page.getByLabel("Inspector", { exact: true });
const order = (page: Page) => insp(page).locator("[data-free] .sg").allTextContents();
const undoCount = async (page: Page) => {
  await page.getByRole("radio", { name: "History" }).click();
  const n = await page.getByRole("list", { name: "History" }).getByRole("listitem").count();
  await page.getByRole("radio", { name: "Notes" }).click();
  return n;
};
const shot = (page: Page, name: string) => page.screenshot({ path: `test-results/visual/${name}.png` });

// req: UI-12, FX-10
test("the chain is a list in the inspector: Trim and Fader · Pan locked, a row per block with its value and where it plays", async ({ page }) => {
  await openDemo(page);
  await header(page, "Lead Vox").click();
  expect(await order(page)).toEqual(["HP", "EQ", "CMP", "DS", "SAT", "DBL"]);
  await expect(insp(page).getByTestId("row-trim")).toContainText("Trim");
  await expect(insp(page).getByTestId("row-fader")).toContainText("Fader · Pan");
  await expect(insp(page).getByTestId("row-sat")).toContainText("Tube · 4 dB");
  // The doubler plays in the choruses only (a bypass lane of the assistant): its row says so.
  await expect(insp(page).getByTestId("row-dbl")).toContainText("Choruses");
  await expect(insp(page).getByTestId("add-block")).toContainText("6 of 12");
  // A row opens its block's editor below the list; the LED switches it, one undo step.
  await insp(page).getByTestId("row-deess").locator(".rname").click();
  await expect(page.getByLabel("De-esser editor")).toBeVisible();
  const steps = await undoCount(page);
  await insp(page).getByRole("button", { name: "De-esser on" }).first().click();
  await expect(insp(page).getByTestId("row-deess")).toHaveClass(/is-off/);
  expect(await undoCount(page)).toBe(steps + 1);
  // The header shows the same labels in the same order (four and "+2").
  await expect(page.getByTestId("marks-Lead Vox").locator("[data-block]")).toHaveText(["HP", "EQ", "CMP", "DS"]);
  await expect(page.getByTestId("marks-Lead Vox").getByTestId("marks-more")).toHaveText("+2");
  await page.waitForTimeout(300);
  await shot(page, "16a-chain");
});

// req: UI-12, FX-10, HIS-1
test("Add block: a menu by family, what cannot be added says why; the block lands before the fader, off, and opens", async ({ page }) => {
  await openDemo(page);
  await header(page, "Kick").click();
  await insp(page).getByTestId("add-block").click();
  const menu = page.getByTestId("add-block-menu");
  for (const fam of ["Dynamics", "Tone", "Space & movement", "Pitch"]) await expect(menu.getByRole("group", { name: fam })).toBeVisible();
  // Kick has a compressor already; two of a kind is the limit, so a second is fine, a third not.
  await expect(menu.getByRole("button", { name: /^Harmonizer:/ })).toBeEnabled();
  await shot(page, "16b-add-block");
  const steps = await undoCount(page).catch(() => 0);
  await insp(page).getByTestId("add-block").click();
  await menu.getByRole("button", { name: /^Exciter:/ }).click();
  await expect(menu).toBeHidden();
  expect(await order(page)).toEqual(["HP", "GT", "EQ", "CMP", "TR", "EXC"]);
  await expect(insp(page).getByTestId("row-exc")).toHaveClass(/is-off/);
  await expect(page.getByLabel("Exciter editor")).toBeVisible();
  expect(await undoCount(page)).toBe(steps + 1);
  // Duplicating the compressor fills its kind: the menu then says why it cannot add a third.
  await insp(page).getByTestId("row-comp").locator(".rgrip").click();
  await page.getByRole("menuitem", { name: "Duplicate" }).click();
  expect(await order(page)).toEqual(["HP", "GT", "EQ", "CMP", "CMP", "TR", "EXC"]);
  await insp(page).getByTestId("add-block").click();
  await expect(menu.getByRole("button", { name: "Compressor: Two already: remove one first" })).toBeDisabled();
  await page.keyboard.press("Escape");
  // One undo step each.
  await page.keyboard.press("ControlOrMeta+z");
  await page.keyboard.press("ControlOrMeta+z");
  expect(await order(page)).toEqual(["HP", "GT", "EQ", "CMP", "TR"]);
});

// req: UI-12, FX-10
test("reordering: drag a row or ⌥↓ on it; Trim and the fader do not move; removing takes the block's lanes too", async ({ page }) => {
  // The move is one edit sent after the drag, never from inside a React state update (it updated the store mid-render).
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  await openDemo(page);
  await header(page, "Lead Vox").click();
  // ⌥↑ on De-esser puts it above the compressor (16c).
  await insp(page).getByTestId("row-deess").locator(".rname").focus();
  await page.keyboard.press("Alt+ArrowUp");
  expect(await order(page)).toEqual(["HP", "EQ", "DS", "CMP", "SAT", "DBL"]);
  // Dragging the saturation by its grip to the top.
  const grip = insp(page).getByTestId("row-sat").locator(".rgrip");
  const target = await insp(page).getByTestId("row-hpf").boundingBox();
  await grip.hover();
  await page.mouse.down();
  await page.mouse.move(target!.x + 20, target!.y + 6, { steps: 6 });
  await expect(insp(page).locator(".rline")).toBeVisible();
  await shot(page, "16c-reorder");
  await page.mouse.up();
  expect(await order(page)).toEqual(["SAT", "HP", "EQ", "DS", "CMP", "DBL"]);
  expect(errors.filter((e) => /while rendering a different component/.test(e))).toEqual([]);
  // The marks follow the list.
  await expect(page.getByTestId("marks-Lead Vox").locator("[data-block]")).toHaveText(["SAT", "HP", "EQ", "DS"]);
  // Removing the doubler takes its bypass lane: the row and the "Choruses" tag go.
  await insp(page).getByTestId("row-dbl").locator(".rname").focus();
  await page.keyboard.press("Backspace");
  expect(await order(page)).toEqual(["SAT", "HP", "EQ", "DS", "CMP"]);
  await page.keyboard.press("ControlOrMeta+z");
  await expect(insp(page).getByTestId("row-dbl")).toContainText("Choruses");
});

// req: FX-11, UI-12
test("a return has its own chain after the processor: a compressor keyed by the vocal ducks it", async ({ page }) => {
  await openDemo(page);
  await strip(page, "Plate").locator(".nm").click();
  const rack = insp(page).getByLabel("Plate · chain");
  await expect(rack).toContainText("After the reverb");
  await expect(insp(page).getByTestId("add-block")).toContainText("0 of 6");
  await insp(page).getByTestId("add-block").click();
  await page.getByTestId("add-block-menu").getByRole("button", { name: /^Compressor:/ }).click();
  await expect(page.getByLabel("Compressor editor")).toBeVisible();
  await page.getByRole("button", { name: /Sidechain of Compressor/ }).click();
  await page.getByRole("menuitem", { name: "Lead Vox", exact: true }).click();
  await insp(page).getByRole("button", { name: "Compressor on" }).first().click();
  await expect(insp(page).getByTestId("row-comp").locator(".sg")).toHaveClass(/key t7/);
  // The return's header and strip show its chain now.
  await expect(page.getByTestId("marks-Plate").locator("[data-block=comp]")).toHaveAttribute("data-state", "on");
  await shot(page, "16p-return-chain");
});

// req: UI-12, UI-11
test("long chains: marks show four and +n; the Overview has a row per kind of block in the project", async ({ page }) => {
  await openDemo(page);
  await expect(strip(page, "Snare").getByRole("button", { name: /more blocks/ })).toHaveText("+2");
  await shot(page, "16t-marks");
  await page.getByRole("radio", { name: "Overview" }).click();
  const rail = page.getByLabel("Mixer rows");
  for (const k of ["High-pass", "Low-pass", "EQ", "Compressor", "Saturation", "Gate", "De-esser", "Dynamic EQ", "Transient", "Bass enhancer", "Width", "Modulation", "Tremolo", "Doubler"]) {
    await expect(rail.locator(".cells > .cl", { hasText: new RegExp(`^${k}`) }).first(), k).toBeVisible();
  }
  await expect(rail.locator(".cells > .cl", { hasText: /^Harmonizer/ })).toHaveCount(0);
  // Lead Vox's de-esser cell, a click bypasses it; absent kinds read "off".
  const ds = strip(page, "Lead Vox").locator("[data-block=deess]");
  await expect(ds).toHaveText("6.5k");
  await ds.click();
  await expect(ds).toHaveAttribute("data-state", "off");
  await expect(strip(page, "Lead Vox").locator(".cl.off", { hasText: "off" }).first()).toBeVisible();
  await shot(page, "16u-overview");
});
