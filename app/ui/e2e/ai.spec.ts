import { expect, test, type Page } from "@playwright/test";
import { expectColumnsAligned } from "./tables";

// The assistant in the Mix workspace (F7 gate) on the mock backend: auto mix with progress and a proposal in review
// (AI-1, AI-2, AI-12), A/B and partial application (AI-2, HIS-3), revision by request with scope (AI-3, AI-5),
// cancel (AI-12), spending cap (AI-16), AI usage (AI-14, AI-15), snapshots (HIS-4), brief and references (PRJ-7).

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}

// req: AI-1, AI-2, AI-12, HIS-3, AI-14
test("auto mix: progress, proposal, A/B, apply all as one step", async ({ page }) => {
  await openDemo(page);
  await expect(page.getByTestId("ai-estimate")).toContainText("$0.13");
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(page.getByText("Auto mix in progress")).toBeVisible();
  await expect(page.getByRole("progressbar", { name: "Progress" })).toBeVisible();
  const panel = page.getByRole("complementary", { name: "Proposal" });
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel).toContainText("Vocal forward");
  await expect(panel).toContainText("4.8 → 2.1");
  // A/B does not change the document.
  await panel.getByRole("radio", { name: "Proposal" }).click();
  await expect(panel).toContainText("Listening to the proposal");
  await expect(page.getByTestId("strip-Lead Vox").locator(".db")).toHaveText("−5.8");
  await panel.getByRole("button", { name: "Apply all" }).click();
  await expect(panel).toBeHidden();
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−7.5");
  // One step in the history, marked as the assistant's.
  await page.getByRole("radio", { name: "History" }).click();
  await expect(page.getByRole("list", { name: "History" }).getByRole("listitem").first()).toContainText("Auto mix applied");
  await page.keyboard.press("Meta+z");
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−6.5");
});

// req: AI-17, AI-18, AI-20, HIS-3
test("structural proposal: new returns and a group, created selectively, removed by one undo", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Auto mix" }).click();
  const panel = page.getByRole("complementary", { name: "Proposal" });
  await expect(panel).toBeVisible({ timeout: 10_000 });
  // What the plan creates comes first, each with what it is, why, and who uses it.
  const fresh = panel.getByLabel("New in this proposal");
  await expect(panel).toContainText("3 of 3 will be created");
  const plate = fresh.getByTestId("new-Vocal plate");
  await expect(plate).toContainText("New");
  await expect(plate).toContainText("reverb return · plate, 1.2 s");
  await expect(plate).toContainText("Lead Vox sends here");
  await expect(fresh.getByTestId("new-Slap")).toContainText("delay return · 110 ms, no repeats");
  const pads = fresh.getByTestId("new-Pads");
  await expect(pads).toContainText("group · Rhodes, Strings");
  await expect(pads).toContainText("2 tracks move into it");
  // Unticked by hand: left out. One of the group's two tracks unticked: not created, and it says why.
  await fresh.getByRole("checkbox", { name: "Create Slap" }).click();
  await expect(fresh.getByTestId("new-Slap")).toContainText("left out");
  await panel.getByTestId("card-Rhodes").getByRole("checkbox", { name: "Apply Rhodes" }).click();
  await expect(pads).toContainText("not created");
  await expect(pads).toContainText("Needs two of its tracks selected: only Strings is.");
  await expect(fresh.getByRole("checkbox", { name: "Create Pads" })).toBeDisabled();
  await expect(panel).toContainText("1 of 3 will be created");
  await panel.getByRole("button", { name: /Apply selected/ }).click();
  await expect(panel).toBeHidden();
  await expect(page.getByTestId("strip-Vocal plate")).toBeVisible();
  await expect(page.getByTestId("strip-Slap")).toHaveCount(0);
  await expect(page.getByTestId("strip-Pads")).toHaveCount(0);
  // One undo removes the new return with the rest of the proposal.
  await page.keyboard.press("Meta+z");
  await expect(page.getByTestId("strip-Vocal plate")).toHaveCount(0);

  // Apply all: the group is created around its tracks.
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await panel.getByRole("button", { name: "Apply all" }).click();
  await expect(page.getByTestId("strip-Pads")).toBeVisible();
  await expect(page.getByTestId("strip-Rhodes").locator(".kind")).toHaveText("→ Pads");
  await expect(page.getByTestId("strip-Slap")).toBeVisible();
  await page.keyboard.press("Meta+z");
  await expect(page.getByTestId("strip-Pads")).toHaveCount(0);

  // AI-18: with the option off the proposal creates nothing.
  const allow = page.getByRole("checkbox", { name: "Allow structural changes" });
  await expect(allow).toBeChecked();
  await allow.click();
  await expect(allow).not.toBeChecked();
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel).not.toContainText("New in this proposal");
});

// req: AI-2, AI-14
test("apply selected tracks only, then discard a revision", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Auto mix" }).click();
  const panel = page.getByRole("complementary", { name: "Proposal" });
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await panel.getByTestId("card-Kick").getByRole("checkbox", { name: "Apply Kick" }).click();
  await expect(panel.getByTestId("card-Kick")).toHaveClass(/is-off/);
  await panel.getByRole("button", { name: /Apply selected/ }).click();
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−6.5");
  await expect(page.getByTestId("strip-Snare").locator(".db")).toHaveText("−8.5");
  // A revision limited to the selected track (AI-5).
  await page.getByLabel("Lead Vox header").click();
  await page.getByRole("radio", { name: /Only Lead Vox/ }).click();
  // One way to start a call (A-063): the request is optional and the same button sends it; Enter is a new line.
  const ask = page.getByLabel("Request to the assistant");
  await expect(page.getByTestId("ask-hint")).toContainText("A request is optional");
  await expect(page.getByRole("button", { name: "Send request" })).toHaveCount(0);
  await ask.fill("vocal louder");
  await ask.press("Enter");
  await ask.pressSequentially("in the verses");
  await expect(ask).toHaveValue("vocal louder\nin the verses");
  await expect(panel).toBeHidden();
  await ask.fill("vocal louder in the verses");
  await expect(page.getByTestId("ask-hint")).toContainText("revise the current mix");
  await expect(page.getByRole("button", { name: "Auto mix" })).toHaveCount(0);
  await page.getByRole("button", { name: "Revise mix" }).click();
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel).toContainText("You asked: “vocal louder in the verses”");
  await expect(panel).toContainText("out of scope");
  await panel.getByRole("button", { name: "Discard" }).click();
  await page.getByRole("dialog", { name: "Discard this proposal?" }).getByRole("button", { name: "Discard" }).click();
  await expect(panel).toBeHidden();
  // Both calls are in AI usage with their outcomes.
  await page.getByRole("button", { name: "AI usage" }).click();
  const calls = page.getByRole("table", { name: "AI calls" });
  await expect(calls).toContainText("Applied in part");
  await expect(calls).toContainText("Discarded");
});

// req: AI-12, AI-15
test("cancel a running call: nothing changes and its cost is counted", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Auto mix" }).click();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByText("Assistant cancelled")).toBeVisible();
  await expect(page.getByRole("complementary", { name: "Proposal" })).toBeHidden();
  await page.getByRole("button", { name: "AI usage" }).click();
  await expect(page.getByTestId("usage-calls")).toHaveText("7");
});

// req: AI-16
test("a request over the spending cap asks first", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "AI usage" }).click();
  await page.getByLabel("Spending cap in USD").fill("0.50");
  await page.getByRole("button", { name: "Set cap" }).click();
  await page.getByRole("button", { name: "Back to Mix" }).click();
  await page.getByRole("button", { name: "Auto mix" }).click();
  const dlg = page.getByRole("dialog", { name: "Spending cap" });
  await expect(dlg).toContainText("Would reach");
  await dlg.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByText("Auto mix in progress")).toBeHidden();
});

// req: AI-14, AI-10
test("AI usage: summary, filters, readable detail and raw JSON", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "AI usage" }).click();
  await expect(page.getByTestId("usage-calls")).toHaveText("6");
  await expect(page.getByTestId("usage-cost")).toHaveText("$0.46");
  await expectColumnsAligned(page.locator(".mtable"));
  await page.getByRole("button", { name: "Outcome" }).click();
  await page.getByRole("menuitem", { name: "Failed" }).click();
  await expect(page.getByRole("table", { name: "AI calls" }).getByRole("row")).toHaveCount(2);
  await page.getByRole("table", { name: "AI calls" }).getByRole("row").nth(1).click();
  const detail = page.getByLabel("Call detail");
  await expect(detail).toContainText("What was sent");
  await expect(detail).toContainText("brighter guitars");
  await expectColumnsAligned(detail.locator(".mtable"));
  await detail.getByRole("radio", { name: "Raw JSON" }).click();
  await detail.getByRole("radio", { name: "Request" }).click();
  await expect(page.getByTestId("raw-json")).toContainText('"messages"');
  await expect(page.getByTestId("raw-json")).not.toContainText(".wav");
});

// req: HIS-4
test("snapshots: save, compare A/B, restore as one step", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("radio", { name: "Snapshots" }).click();
  await page.getByRole("button", { name: "Save snapshot" }).click();
  await page.getByLabel("Snapshot name").fill("Rough balance");
  await page.getByLabel("Snapshot name").press("Enter");
  await expect(page.getByTestId("snapshot-Rough balance")).toBeVisible();
  await page.getByTestId("strip-Kick").locator(".fader").dblclick();
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("0.0");
  await page.getByRole("radio", { name: /B · Rough balance/ }).click();
  await expect(page.getByRole("radio", { name: /B · Rough balance/ })).toHaveAttribute("aria-checked", "true");
  await page.getByTestId("snapshot-Rough balance").getByRole("button", { name: "Restore" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Restore" }).click();
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−6.5");
});

// req: PRJ-7, AI-11
test("brief and references; the missing-key state offers the offline demo", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Edit brief" }).click();
  const dlg = page.getByRole("dialog", { name: "Project settings" });
  await dlg.getByLabel("Genre").fill("dream pop");
  await dlg.getByRole("button", { name: /Add a finished song/ }).click();
  await expect(dlg).toContainText("Reference A.mp3");
  await dlg.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("complementary", { name: "Assistant" })).toContainText("dream pop");
  // The reference is shown by name in the assistant; it opens the same settings.
  await page.getByRole("complementary", { name: "Assistant" }).getByRole("button", { name: /Reference A\.mp3/ }).click();
  await expect(page.getByRole("dialog", { name: "Project settings" })).toBeVisible();
  await page.getByRole("dialog", { name: "Project settings" }).getByRole("button", { name: "Cancel" }).click();
  await page.evaluate(() => (window as unknown as { __stemwiseMock: { aiSetKey(k: string | null): Promise<void> } }).__stemwiseMock.aiSetKey(null));
  await expect(page.getByText("The assistant is not set up")).toBeVisible();
  await expect(page.getByRole("button", { name: "Auto mix" })).toBeDisabled();
  await page.getByRole("button", { name: "Use offline demo" }).click();
  await expect(page.getByTestId("ai-estimate")).toContainText("offline demo");
  await expect(page.getByRole("button", { name: "Auto mix" })).toBeEnabled();
});

// req: AI-3, AI-5
test("the request row keeps its button inside the panel when the selected track has a long name", async ({ page }) => {
  await openDemo(page);
  // Real projects take track names from the files: far longer than the demo's (issue #6).
  const name = "Lead Vocal Double Left Take 3 Comp Final";
  await page.getByLabel("Lead Vox header").locator(".names").dblclick();
  await page.getByLabel("Track name").fill(name);
  await page.getByLabel("Track name").press("Enter");
  await page.getByLabel(`${name} header`).click();
  await page.getByLabel("Request to the assistant").fill("vocal louder in the verses");
  const button = page.getByRole("button", { name: "Revise mix" });
  const foot = page.locator(".foot", { has: button });
  const inside = async () => {
    const [b, f] = [await button.boundingBox(), await foot.boundingBox()];
    return b!.x >= f!.x && b!.x + b!.width <= f!.x + f!.width;
  };
  expect(await inside()).toBe(true);
  // The scope switch gives way instead: the name is cut, the whole of it stays reachable.
  const one = page.getByRole("radio", { name: `Only ${name}` });
  await expect(one).toHaveAttribute("title", name);
  expect(await one.locator(".ell").evaluate((e) => e.scrollWidth > e.clientWidth)).toBe(true);
  await one.click();
  await expect(one).toHaveAttribute("aria-checked", "true");
  expect(await inside()).toBe(true);
});

// req: UI-4, AI-2
test("while a proposal takes the side columns, their switches leave the bar and the mixer's still works", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(page.getByRole("complementary", { name: "Proposal" })).toBeVisible({ timeout: 10_000 });
  const sw = (name: string) => page.getByRole("group", { name: "Panels" }).getByRole("button", { name: `${name} panel` });
  await expect(sw("Inspector")).toHaveCount(0);
  await expect(sw("Assistant")).toHaveCount(0);
  await sw("Mixer").click();
  await expect(page.getByRole("region", { name: "Mixer" })).toBeHidden();
  await expect(page.getByRole("complementary", { name: "Proposal" })).toBeVisible();
});

// req: AI-23, AI-2, HIS-3
test("proposals received earlier: listed with when and what became of them, reviewed and applied after a confirmation", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("radio", { name: "Proposals" }).click();
  const list = page.getByRole("list", { name: "Proposals received" });
  // Every call that came back with a proposal, newest first; the failed one has none.
  await expect(list.getByRole("listitem")).toHaveCount(4);
  await expect(list.getByRole("listitem").first()).toContainText("10 Nov 2026");
  await expect(list).not.toContainText("brighter guitars");
  const discarded = list.getByRole("listitem").filter({ hasText: "Discarded" });
  await expect(discarded).toContainText("Auto mix");
  await discarded.getByRole("button", { name: /Review/ }).click();
  // Back in review, on the mix as it is now: said when it was received; closing loses nothing.
  const panel = page.getByRole("complementary", { name: "Proposal" });
  await expect(panel).toBeVisible();
  await expect(panel.getByTestId("reopened-note")).toContainText("Received 10 Nov 2026");
  await expect(panel.getByRole("button", { name: "Discard" })).toHaveCount(0);
  // Applying it changes the current mix: an explicit confirmation first, and Cancel changes nothing.
  await panel.getByRole("button", { name: "Apply all" }).click();
  const confirm = page.getByRole("dialog", { name: "Apply this proposal to your current mix?" });
  await expect(confirm).toContainText("replaces the current settings");
  await confirm.getByRole("button", { name: "Cancel" }).click();
  await expect(panel).toBeVisible();
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−6.5");
  await panel.getByRole("button", { name: "Apply all" }).click();
  await confirm.getByRole("button", { name: "Apply all" }).click();
  await expect(panel).toBeHidden();
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−7.5");
  // The same call now says applied; no new call was made, and it is one step of the history.
  await page.getByRole("radio", { name: "Proposals" }).click();
  await expect(list.getByRole("listitem")).toHaveCount(4);
  await expect(list.getByRole("listitem").filter({ hasText: "Discarded" })).toHaveCount(0);
  // Changed by hand afterwards, the proposal that was applied can be applied again.
  await page.keyboard.press("Meta+z");
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−6.5");
  const applied = list.getByRole("listitem").filter({ hasText: "Auto mix" }).first();
  await applied.getByRole("button", { name: /Review/ }).click();
  await expect(panel).toBeVisible();
  await panel.getByRole("button", { name: "Close review" }).click();
  await expect(panel).toBeHidden();
  await page.getByRole("radio", { name: "Proposals" }).click();
  await expect(list.getByRole("listitem").filter({ hasText: "Discarded" })).toHaveCount(0);
  await expect(page.getByTestId("strip-Kick").locator(".db")).toHaveText("−6.5");
});

// req: AI-25, AI-26, AI-27, UI-13, HIS-3
test("creative ideas: off by default, ticked one by one, listened to alone, applied with the plan in one step", async ({ page }) => {
  await openDemo(page);
  const allow = page.getByRole("checkbox", { name: "Allow creative ideas" });
  await expect(allow).toBeChecked();
  await page.getByRole("button", { name: "Auto mix" }).click();
  const panel = page.getByRole("complementary", { name: "Proposal" });
  await expect(panel).toBeVisible({ timeout: 10_000 });
  // AI-25, design 4c: the plan's own new block is a row of the vocal's diff, marked new.
  await expect(panel).toContainText("De-esser · new block");
  await expect(panel).toContainText("6.5 kHz");
  // Three ideas, each with why, where and what it adds; none ticked.
  const ideas = panel.getByLabel("Creative ideas");
  await expect(ideas.locator(".icard")).toHaveCount(3);
  await expect(panel).toContainText("0 of 3 ticked");
  const chorus = ideas.getByTestId("idea-i1");
  await expect(chorus).toContainText("Chorus on the keys");
  await expect(chorus).toContainText("Rhodes · Modulation, chorus 1/4, 35 % · new block");
  await expect(chorus).toContainText("Choruses");
  await chorus.getByRole("checkbox", { name: "Apply idea Chorus on the keys" }).click();
  await expect(chorus).toHaveClass(/\bon\b/);
  await expect(panel).toContainText("1 of 3 ticked");
  await page.screenshot({ path: "test-results/visual/4c-creative-ideas.png" });
  // Design 4d: "Listen" plays the proposal with that idea alone, until pressed again.
  const halo = ideas.getByTestId("idea-i3");
  await halo.getByRole("button", { name: "Listen to idea Octave halo in the bridge" }).click();
  await expect(halo.getByRole("button", { name: /Listen to idea/ })).toHaveText("Listening");
  await expect(halo).toHaveClass(/is-play/);
  await expect(panel).toContainText("Listening to the proposal");
  await page.screenshot({ path: "test-results/visual/4d-listen-to-one-idea.png" });
  await halo.getByRole("button", { name: /Listen to idea/ }).click();
  await expect(halo.getByRole("button", { name: /Listen to idea/ })).toHaveText("Listen");
  // Apply all: the plan with the ticked idea, one step; the unticked ones are not created.
  await panel.getByRole("button", { name: "Apply all" }).click();
  await expect(panel).toBeHidden();
  await page.getByRole("radio", { name: "History" }).click();
  await expect(page.getByRole("list", { name: "History" }).getByRole("listitem").first()).toContainText("idea Chorus on the keys");
  await expect(page.getByTestId("strip-Halo")).toHaveCount(0);
  await page.getByLabel("Rhodes header").click();
  const insp = page.getByLabel("Inspector", { exact: true });
  await expect(insp.getByTestId("row-mod2")).toBeVisible();
  // UI-13: the track's motivation names the idea.
  await expect(insp).toContainText("Idea “Chorus on the keys”");
  await page.keyboard.press("Meta+z");
  await expect(insp.getByTestId("row-mod2")).toHaveCount(0);

  // AI-27: with the option off the proposal has no ideas.
  await page.getByRole("radio", { name: "Notes" }).click();
  await allow.click();
  await expect(allow).not.toBeChecked();
  await page.getByRole("button", { name: "Auto mix" }).click();
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel).not.toContainText("Creative ideas");
});
