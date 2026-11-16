import { expect, test } from "@playwright/test";

// req: UI-2
test("the UI runs in the browser on the mock backend", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("backend-kind")).toHaveText(/mock backend/);
});

// req: ENG-3, HIS-2, TRN-1
test("vertical slice: open a project, play, move a fader, undo", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  const strip = page.getByTestId("strip-Bass DI");
  await expect(strip).toBeVisible();
  await page.getByRole("button", { name: "Play" }).click();
  await expect(page.getByTestId("position")).not.toHaveText("0:00.00");
  const fader = strip.locator(".fader");
  await fader.focus();
  await page.keyboard.press("ArrowDown");
  await expect(strip.locator(".db")).not.toHaveText("−7.2");
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(strip.locator(".db")).toHaveText("−7.2");
});
