import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

// Assistant setup (design 14a–14k) on the mock backend: connecting an OpenRouter account from the browser (AI-24),
// the credit step, the manual key path (AI-11), the provider's refusals in the assistant column (UI-6), keyboard and
// accessibility of the dialog (UI-7).

type Mock = {
  connectOutcome: string;
  connectDelay: number;
  opened: string[];
  simulateRefusal(kind: "credit" | "cap" | "revoked" | null): void;
  aiSetKey(key: string | null): Promise<void>;
};
const mock = <T>(page: Page, f: (m: Mock) => T) => page.evaluate(`(${f.toString()})(window.__stemwiseMock)`) as Promise<T>;
const shot = (page: Page, name: string) => page.screenshot({ path: `test-results/visual/${name}.png` });
const dialog = (page: Page) => page.getByRole("dialog", { name: "Set up the assistant" });

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}

// req: AI-24, AI-11, UI-7
test("first launch: connect in the browser, add credit, ready", async ({ page }) => {
  await page.goto("/?first-launch");
  // 1b: the invitation in place of the recent projects, with its three steps.
  await expect(page.getByRole("heading", { name: "Set up the assistant" })).toBeVisible();
  await expect(page.getByText("Add a little credit")).toBeVisible();
  await shot(page, "1b-first-launch");
  await page.getByRole("button", { name: "Set up the assistant" }).click();
  const dlg = dialog(page);
  await expect(dlg.getByText("What is OpenRouter?")).toBeVisible();
  await expect(dlg.getByRole("listitem").filter({ hasText: "Connect OpenRouter" })).toHaveAttribute("aria-current", "step");
  await expect(dlg.getByRole("button", { name: "Connect with OpenRouter" })).toBeFocused();
  expect((await new AxeBuilder({ page }).include("[role=dialog]").analyze()).violations).toEqual([]);
  await shot(page, "14a-connect");
  // 14b: the browser opens on the approval page; the dialog waits and can open the page again.
  await mock(page, (m) => void (m.connectDelay = 1200));
  await dlg.getByRole("button", { name: "Connect with OpenRouter" }).click();
  await expect(dlg.getByRole("status")).toContainText("Waiting for your approval in the browser");
  await dlg.getByRole("button", { name: "Open the page again" }).click();
  expect(await mock(page, (m) => m.opened)).toEqual(["auth", "auth"]);
  await shot(page, "14b-waiting");
  // 14d: approved. Costs at the model's prices, the credits page, and the limit said plainly.
  await expect(dlg.getByText("Add credit on OpenRouter")).toBeVisible({ timeout: 5000 });
  await expect(dlg.getByTestId("typical-mix")).toHaveText("≈ $0.12");
  await expect(dlg).toContainText("≈ 41 mixes");
  await expect(dlg).toContainText("Stemwise cannot see your balance");
  await dlg.getByRole("button", { name: "Open the credits page" }).click();
  expect(await mock(page, (m) => m.opened.at(-1))).toBe("credits");
  expect((await new AxeBuilder({ page }).include("[role=dialog]").analyze()).violations).toEqual([]);
  await shot(page, "14d-credit");
  await dlg.getByRole("button", { name: "I have added credit" }).click();
  // 14e: where the key is and how to disconnect.
  await expect(dlg.getByTestId("setup-ready")).toContainText("Key kept in the macOS keychain");
  await expect(dlg).toContainText("delete the key named “Stemwise”");
  await shot(page, "14e-ready");
  await dlg.getByRole("button", { name: "Done" }).click();
  await expect(dlg).toBeHidden();
  // With a key the invitation has done its job.
  await expect(page.getByText("No recent projects yet.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Set up the assistant" })).toBeHidden();
});

// req: AI-24, AI-11
test("a declined connection says so and the manual path takes a pasted key", async ({ page }) => {
  await page.goto("/?first-launch");
  await page.getByRole("button", { name: "Set up the assistant" }).click();
  const dlg = dialog(page);
  await mock(page, (m) => void ((m.connectOutcome = "denied"), (m.connectDelay = 50)));
  await dlg.getByRole("button", { name: "Connect with OpenRouter" }).click();
  // 14f
  await expect(dlg.getByTestId("connect-failed")).toContainText("The connection was not approved");
  await expect(dlg.getByTestId("connect-failed")).toContainText("Nothing was saved");
  await shot(page, "14f-not-approved");
  await mock(page, (m) => void (m.connectOutcome = "expired"));
  await dlg.getByRole("button", { name: "Try again" }).click();
  await expect(dlg.getByTestId("connect-failed")).toContainText("longer than 10 minutes");
  // 14g, 14h: three numbered steps with their links, then the field; the key is never shown.
  await dlg.getByRole("button", { name: "Paste a key instead" }).click();
  const field = dlg.getByLabel("OpenRouter key");
  await expect(field).toBeFocused();
  await dlg.getByRole("button", { name: "Open keys page" }).click();
  expect(await mock(page, (m) => m.opened.at(-1))).toBe("keys");
  await expect(field).toHaveAttribute("type", "password");
  await expect(dlg.getByRole("button", { name: "Save key" })).toBeDisabled();
  await shot(page, "14g-manual");
  await field.fill("sk-or-v1-pasted");
  await field.press("Enter");
  await expect(dlg.getByText("Add credit on OpenRouter")).toBeVisible();
  await dlg.getByRole("button", { name: "Skip for now" }).click();
  await expect(dlg.getByTestId("setup-ready")).toBeVisible();
  // Escape closes; the key stayed.
  await page.keyboard.press("Escape");
  await expect(dlg).toBeHidden();
  await expect(page.getByText("No recent projects yet.")).toBeVisible();
});

// req: AI-24
test("cancelling or leaving the dialog stops waiting for the browser", async ({ page }) => {
  await page.goto("/?first-launch");
  await page.getByRole("button", { name: "Set up the assistant" }).click();
  const dlg = dialog(page);
  await mock(page, (m) => void (m.connectDelay = 600));
  await dlg.getByRole("button", { name: "Connect with OpenRouter" }).click();
  await dlg.getByRole("button", { name: "Cancel" }).click();
  await expect(dlg.getByRole("button", { name: "Connect with OpenRouter" })).toBeVisible();
  // Leaving while waiting: no key arrives later behind the user's back.
  await dlg.getByRole("button", { name: "Connect with OpenRouter" }).click();
  await dlg.getByRole("button", { name: "Not now" }).click();
  await page.waitForTimeout(900);
  await expect(page.getByRole("button", { name: "Set up the assistant" })).toBeVisible();
  // Skipping the invitation, or choosing the offline demo, leaves the recent projects in its place.
  await page.getByRole("button", { name: "Use offline demo" }).click();
  await expect(page.getByText("Offline demo on")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Recent projects" })).toBeVisible();
});

// req: AI-24, UI-6
test("the provider's refusals are told apart in the assistant column, each with its way out", async ({ page }) => {
  await openDemo(page);
  const banner = page.getByTestId("refusal-banner");
  const dismiss = () => page.getByRole("alert").filter({ hasText: "could not answer" }).getByRole("button", { name: "Dismiss" }).click();
  // 14i: no credit.
  await mock(page, (m) => m.simulateRefusal("credit"));
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(banner).toContainText("Your OpenRouter credit has run out");
  await expect(banner).toContainText("nothing in your mix changed");
  await shot(page, "14i-no-credit");
  await dismiss();
  await banner.getByRole("button", { name: "Add credit…" }).click();
  expect(await mock(page, (m) => m.opened.at(-1))).toBe("credits");
  // 14j: the key's own limit.
  await mock(page, (m) => m.simulateRefusal("cap"));
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(banner).toContainText("The key has reached its own limit");
  await dismiss();
  await banner.getByRole("button", { name: "Open keys page" }).click();
  expect(await mock(page, (m) => m.opened.at(-1))).toBe("keys");
  // 14k: a key the provider no longer takes; connecting again clears the banner.
  await mock(page, (m) => m.simulateRefusal("revoked"));
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(banner).toContainText("OpenRouter no longer accepts this key");
  await dismiss();
  await mock(page, (m) => void (m.simulateRefusal(null), (m.connectDelay = 50)));
  await banner.getByRole("button", { name: "Connect with OpenRouter" }).click();
  await dialog(page).getByRole("button", { name: "Connect with OpenRouter" }).click();
  await expect(dialog(page).getByText("Add credit on OpenRouter")).toBeVisible();
  // In a project the cap is set right here.
  const cap = dialog(page).getByRole("switch", { name: /Ask me before this project spends more than/ });
  if ((await cap.getAttribute("aria-checked")) !== "true") await cap.click();
  await expect(cap).toHaveAttribute("aria-checked", "true");
  await dialog(page).getByRole("button", { name: "Skip for now" }).click();
  await expect(dialog(page)).toContainText("$2.00 for this project");
  await dialog(page).getByRole("button", { name: "Done" }).click();
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(page.getByText("Auto mix in progress")).toBeVisible();
  await expect(banner).toBeHidden();
});

// req: AI-11, SET-1
test("settings: the account row connects, pastes and disconnects", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Settings (⌘,)" }).click();
  await page.getByRole("button", { name: "AI" }).click();
  await expect(page.getByTestId("key-state")).toHaveText("Stored in the keychain");
  await page.getByRole("button", { name: "Add credit on OpenRouter" }).click();
  expect(await mock(page, (m) => m.opened.at(-1))).toBe("credits");
  await page.getByRole("button", { name: "Disconnect" }).click();
  const confirm = page.getByRole("dialog", { name: "Disconnect OpenRouter?" });
  await expect(confirm).toContainText("stays valid in your OpenRouter account until you delete it there");
  await confirm.getByRole("button", { name: "Disconnect" }).click();
  await expect(page.getByTestId("key-state")).toHaveText("No key");
  await page.getByRole("button", { name: "Paste a key instead" }).click();
  await expect(dialog(page).getByLabel("OpenRouter key")).toBeVisible();
  await dialog(page).getByRole("button", { name: "Connect with OpenRouter instead" }).click();
  await mock(page, (m) => void (m.connectDelay = 50));
  await dialog(page).getByRole("button", { name: "Connect with OpenRouter" }).click();
  await expect(dialog(page).getByText("Add credit on OpenRouter")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("key-state")).toHaveText("Stored in the keychain");
});

// req: UI-6, UI-5
test("at the minimum window the banners of the assistant column keep their buttons inside", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await openDemo(page);
  const inside = async (banner: ReturnType<Page["getByTestId"]>) => {
    const box = (await banner.boundingBox())!;
    for (const b of await banner.getByRole("button").all()) {
      const r = (await b.boundingBox())!;
      expect(r.x >= box.x && r.x + r.width <= box.x + box.width, `“${await b.innerText()}” stays inside its banner`).toBe(true);
    }
  };
  // 13e: no key. Two buttons that do not fit side by side go one under the other.
  await mock(page, (m) => void m.aiSetKey(null));
  await expect(page.getByTestId("key-banner")).toBeVisible();
  await inside(page.getByTestId("key-banner"));
  await mock(page, (m) => void m.aiSetKey("sk-or-back"));
  // 14i–14k and 13f share the shape.
  for (const kind of ["credit", "cap", "revoked"] as const) {
    await page.evaluate((k) => (window as unknown as { __stemwiseMock: Mock }).__stemwiseMock.simulateRefusal(k), kind);
    await page.getByRole("button", { name: "Auto mix" }).click();
    await expect(page.getByTestId("refusal-banner")).toBeVisible();
    await inside(page.getByTestId("refusal-banner"));
    await page.getByRole("alert").filter({ hasText: "could not answer" }).getByRole("button", { name: "Dismiss" }).click();
  }
});
