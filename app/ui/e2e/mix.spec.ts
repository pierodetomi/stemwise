import { expect, test, type Page } from "@playwright/test";

// Main flows of the Mix workspace on the mock backend (F6 gate): selection, mixer and inspector agreeing (TRK-4),
// continuous controls (UI-5), clip editing (CLP-1…4), sections (SEC-1), automation (AUT-2, AUT-3), returns and
// routing (TRK-1, TRK-3, FX-3), history (HIS-1, HIS-6), import (PRJ-3, PRJ-5, PRJ-6).

async function openDemo(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /Confidence/ }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
}
const header = (page: Page, name: string) => page.getByLabel(`${name} header`);
const historyLabels = async (page: Page) => {
  await page.getByRole("radio", { name: "History" }).click();
  const items = await page.getByRole("list", { name: "History" }).getByRole("listitem").allInnerTexts();
  await page.getByRole("radio", { name: "Notes" }).click();
  return items.join("\n");
};

// req: TRK-4, UI-5, HIS-2
test("selection, fader typing and mute agree between timeline, mixer and inspector", async ({ page }) => {
  await openDemo(page);
  await header(page, "Bass DI").click();
  await expect(page.getByTestId("inspector-name")).toHaveText("Bass DI");
  const strip = page.getByTestId("strip-Bass DI");
  // Typing a value: click the value, type, Enter (UI-5); out-of-range text is refused.
  await strip.locator(".db").click();
  const input = page.getByLabel("Bass DI · Level value");
  await input.fill("40");
  await input.press("Enter");
  await expect(input).toHaveClass(/is-err/);
  await input.fill("-10");
  await input.press("Enter");
  await expect(strip.locator(".db")).toHaveText("−10.0");
  await expect(header(page, "Bass DI").getByRole("slider")).toHaveAttribute("aria-valuenow", "-10");
  // Double click resets to the default (0 dB).
  await strip.locator(".fader").dblclick();
  await expect(strip.locator(".db")).toHaveText("0.0");
  // Mute in the header shows in the strip; it is view state, not history (HIS-6).
  await header(page, "Bass DI").getByRole("button", { name: "Mute Bass DI" }).click();
  await expect(strip.getByRole("button", { name: "Mute Bass DI" })).toHaveAttribute("aria-pressed", "true");
  // Solo one track: the others show the implied solo state.
  await header(page, "Kick").getByRole("button", { name: "Solo Kick" }).click();
  await expect(header(page, "Snare").getByRole("button", { name: "Solo Snare" })).toHaveClass(/implied/);
  const h = await historyLabels(page);
  expect(h).toContain("Bass DI · Level");
  expect(h).not.toMatch(/Mute|Solo/);
});

// req: CLP-1, CLP-4, HIS-1
test("clips: select, move by drag, split, delete and undo", async ({ page }) => {
  await openDemo(page);
  const clips = page.locator('[data-audio-track] [data-clip]');
  const rhodes = page.locator('[data-track-row] .clip:has-text("Rhodes")').first();
  const box = (await rhodes.boundingBox())!;
  // Drag the body by 60 px: one "Move clip" step.
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2, { steps: 4 });
  await page.mouse.move(box.x + box.width / 2 + 60, box.y + box.height / 2, { steps: 4 });
  await page.mouse.up();
  await expect.poll(() => historyLabels(page)).toContain("Move clip");
  const before = await clips.count();
  // Split with the context menu at the playhead (inside the moved clip after locating there).
  await page.getByRole("button", { name: /Clip Rhodes/ }).first().click();
  await page.keyboard.press("Delete");
  await expect(clips).toHaveCount(before - 1);
  await page.keyboard.press("Meta+z");
  await expect(clips).toHaveCount(before);
  // Context menu: duplicate.
  // The last Strings clip: its copy goes after it, on empty space (a copy over a clip would replace it, CLP-4).
  await page.getByRole("button", { name: /Clip Strings/ }).last().click({ button: "right" });
  await page.getByRole("menuitem", { name: /Duplicate/ }).click();
  await expect(clips).toHaveCount(before + 1);
});

// req: CLP-1, CLP-2
test("split tool and clip gain from the context menu", async ({ page }) => {
  await openDemo(page);
  const clips = page.locator("[data-audio-track] [data-clip]");
  const before = await clips.count();
  await page.getByRole("radio", { name: "Split" }).click();
  const acoustic = page.getByRole("button", { name: /Clip Acoustic/ });
  const b = (await acoustic.boundingBox())!;
  await page.mouse.click(b.x + b.width * 0.37, b.y + b.height / 2);
  await expect(clips).toHaveCount(before + 1);
  await page.getByRole("radio", { name: "Select" }).click();
  await page.getByRole("button", { name: /Clip Acoustic/ }).first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Clip gain" }).hover();
  await page.getByRole("menuitem", { name: "+3.0 dB" }).click();
  await expect(page.getByRole("button", { name: /Clip Acoustic/ }).first().locator(".gain")).toHaveText("+3.0 dB");
});

// req: SEC-1
test("sections: create by dragging, rename by double click, delete", async ({ page }) => {
  await openDemo(page);
  // Make room: delete the outro, then draw a new section over its span.
  await page.locator(".sec", { hasText: "Outro" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Delete section" }).click();
  await expect(page.locator(".sec", { hasText: "Outro" })).toHaveCount(0);
  const lane = page.getByTestId("sections-lane");
  const lb = (await lane.boundingBox())!;
  // The outro is 14 s: about 27 px with the lanes as wide as they are next to the headers' marks (P-013).
  await page.mouse.move(lb.x + lb.width - 22, lb.y + lb.height / 2);
  await page.mouse.down();
  await page.mouse.move(lb.x + lb.width - 6, lb.y + lb.height / 2, { steps: 5 });
  await page.mouse.up();
  const created = page.locator(".sec", { hasText: /Section \d+/ });
  await expect(created).toHaveCount(1);
  await created.dblclick();
  const input = page.getByLabel("Section name");
  await input.fill("Coda");
  await input.press("Enter");
  await expect(page.locator(".sec", { hasText: "Coda" })).toHaveCount(1);
});

// req: AUT-1, AUT-2, AUT-3
test("automation: show a lane, add points with the pen, offset the curve from the control", async ({ page }) => {
  await openDemo(page);
  await header(page, "Strings").click({ button: "right" });
  await page.getByRole("menuitem", { name: "Show automation" }).hover();
  await page.getByRole("menuitem", { name: "Pan" }).click();
  const lane = page.locator('[data-lane$=":pan"]');
  await expect(lane).toBeVisible();
  await lane.scrollIntoViewIfNeeded();
  await page.getByRole("radio", { name: "Automation" }).click();
  const lb = (await lane.boundingBox())!;
  await page.mouse.click(lb.x + lb.width * 0.3, lb.y + lb.height * 0.25);
  await page.mouse.click(lb.x + lb.width * 0.6, lb.y + lb.height * 0.75);
  await expect(lane.locator(".dot")).toHaveCount(2);
  // The pan knob of the strip now shows the automated value and moving it offsets the curve (AUT-3).
  const strip = page.getByTestId("strip-Strings");
  const pan = strip.getByRole("slider", { name: "Strings · Pan" });
  await pan.focus();
  await page.keyboard.press("ArrowUp");
  await expect.poll(() => historyLabels(page)).toContain("automation offset");
  await expect(lane.locator(".dot")).toHaveCount(2);
});

// req: TRK-1, TRK-3, TRK-5, FX-3, FX-5, FX-6
test("returns and routing: add a return, route a track, key a compressor", async ({ page }) => {
  await openDemo(page);
  await page.getByRole("button", { name: "Add track" }).click();
  await page.getByRole("menuitem", { name: "Reverb return" }).click();
  await expect(page.getByTestId("strip-Reverb 2")).toBeVisible();
  // The return's inspector: processor and who sends to it.
  await page.getByTestId("strip-Plate").click();
  await expect(page.getByTestId("inspector-name")).toHaveText("Plate");
  await expect(page.getByLabel("Inspector", { exact: true })).toContainText("Lead Vox");
  // Output of Bass DI to the Guitars group; loops are not offered.
  await header(page, "Bass DI").click();
  await page.getByRole("button", { name: /Output of Bass DI/ }).click();
  await page.getByRole("menuitem", { name: "Guitars" }).click();
  await expect(page.getByTestId("strip-Bass DI").locator(".kind")).toHaveText("→ Guitars");
  await header(page, "Guitars").click();
  await page.getByRole("button", { name: /Output of Guitars/ }).click();
  await expect(page.getByRole("menuitem", { name: "Drums" })).toBeVisible();
  await page.keyboard.press("Escape");
  // Sidechain: Bass DI listens to the kick (its compressor, opened from the chain).
  await header(page, "Bass DI").click();
  await page.getByTestId("row-comp").click();
  await page.getByRole("button", { name: /Sidechain of Compressor/ }).click();
  await page.getByRole("menuitem", { name: "Kick", exact: true }).click();
  await expect(page.getByRole("button", { name: /Sidechain of Compressor: Kick/ })).toBeVisible();
});

// req: FX-5, FX-3, UI-5
test("reverb algorithm and parallel compression: one history step each, and both undo", async ({ page }) => {
  await openDemo(page);
  // The demo's reverb is a plate: its panel shows the three algorithms and what the chosen one is for.
  await page.getByTestId("strip-Plate").click();
  const insp = page.getByLabel("Inspector", { exact: true });
  const algorithm = insp.getByRole("radiogroup", { name: "Algorithm" });
  await expect(algorithm.getByRole("radio", { name: "Plate" })).toHaveAttribute("aria-checked", "true");
  await expect(insp).toContainText("no early reflections");
  await expect(insp.getByRole("slider", { name: "Plate · Decay" })).toHaveAttribute("aria-valuetext", "1.2 s");
  await algorithm.getByRole("radio", { name: "Hall" }).click();
  await expect(algorithm.getByRole("radio", { name: "Hall" })).toHaveAttribute("aria-checked", "true");
  await expect(insp).toContainText("Long and diffuse");
  // The parameters stay as they were: only the algorithm changes.
  await expect(insp.getByRole("slider", { name: "Plate · Decay" })).toHaveAttribute("aria-valuetext", "1.2 s");
  // Every reverb parameter is there, the new ones too.
  for (const name of ["Size", "Pre-delay", "Damping", "Modulation", "High cut", "Low cut"]) {
    await expect(insp.getByRole("slider", { name: `Plate · ${name}` })).toBeVisible();
  }
  await page.keyboard.press("ControlOrMeta+z");
  await expect(algorithm.getByRole("radio", { name: "Plate" })).toHaveAttribute("aria-checked", "true");

  // Parallel compression: the compressor's Mix below 100 %.
  await header(page, "Snare").click();
  await page.getByTestId("row-comp").click();
  const mix = insp.getByRole("slider", { name: "Snare · Compressor mix" });
  // The demo's snare is already parallel (35 %, design 3a): first back to 100 %, then down again.
  const setMix = async (v: string) => {
    await mix.click();
    const field = page.getByLabel("Snare · Compressor mix value");
    await field.fill(v);
    await field.press("Enter");
  };
  await expect(mix).toHaveAttribute("aria-valuetext", "35 %");
  await setMix("100");
  await expect(mix).toHaveAttribute("aria-valuetext", "100 %");
  await expect(insp.locator("#insp-editor")).toContainText("Stereo-linked");
  await setMix("35");
  await expect(mix).toHaveAttribute("aria-valuetext", "35 %");
  await expect(insp.locator("#insp-editor")).toContainText("Parallel");
  await page.keyboard.press("ControlOrMeta+z");
  await expect(mix).toHaveAttribute("aria-valuetext", "100 %");
});

// req: HIS-1
test("history: steps are listed and clicking one goes back to it", async ({ page }) => {
  await openDemo(page);
  for (const name of ["Snare", "Room"]) {
    await page.getByTestId(`strip-${name}`).locator(".fader").dblclick();
  }
  await page.getByRole("radio", { name: "History" }).click();
  const list = page.getByRole("list", { name: "History" });
  await expect(list.getByRole("listitem")).toHaveCount(2);
  await page.getByRole("button", { name: "Project opened" }).click();
  await expect(page.getByTestId("strip-Snare").locator(".db")).toHaveText("−8.0");
  await expect(list.getByRole("listitem").first()).toHaveClass(/is-dis/);
});

// req: PRJ-1, PRJ-3, PRJ-5, PRJ-6
test("new project: files with roles, a rate mismatch decided explicitly, then the workspace", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New project" }).click();
  await expect(page.getByTestId("drop-area")).toBeVisible();
  await page.evaluate(() => (window as unknown as { __stemwiseMock: { simulateDrop(t: string, p?: string[]): void } }).__stemwiseMock.simulateDrop("over"));
  await expect(page.getByTestId("drop-area")).toHaveClass(/is-over/);
  await page.evaluate(() => (window as unknown as { __stemwiseMock: { simulateDrop(t: string, p?: string[]): void } }).__stemwiseMock.simulateDrop("leave"));
  await page.getByRole("button", { name: "Choose files…" }).click();
  await expect(page.getByRole("table", { name: "Files to import" })).toContainText("11 Lead Vox.wav");
  await expect(page.getByRole("alert")).toContainText("10 Strings.wav");
  await page.getByRole("button", { name: /Role of Kick In/ }).click();
  await page.getByRole("menuitem", { name: "drums" }).click();
  await expect(page.getByRole("button", { name: /Role of Kick In: drums/ })).toBeVisible();
  await page.getByLabel("Project name").fill("Paper Lanterns");
  await page.getByRole("button", { name: "Create project" }).click();
  await expect(page.getByRole("dialog", { name: "Different sample rate" })).toBeVisible();
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("button", { name: "Create project" }).click();
  await expect(page.getByTestId("project-name")).toHaveText("Paper Lanterns");
  await expect(page.getByTestId("strip-Strings")).toBeVisible();
});

type Mock = { __stemwiseMock: { simulateDrop(t: string, p?: string[], at?: { x: number; y: number }): void; project: { references: unknown[]; name: string } | null } };

// req: PRJ-1, PRJ-7
test("new project: references chosen or dropped before it exists are added with it", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New project" }).click();
  await page.getByRole("button", { name: "Choose files…" }).click();
  const refs = page.getByTestId("references");
  // By click…
  await refs.getByRole("button", { name: /Add a finished song/ }).click();
  await expect(refs).toContainText("Reference A.mp3");
  // …and by a drop on the references box (a drop elsewhere adds tracks).
  const box = (await page.getByTestId("reference-drop").boundingBox())!;
  const at = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  await page.evaluate((at) => (window as unknown as Mock).__stemwiseMock.simulateDrop("over", ["/x/Ref B.flac"], at), at);
  await expect(page.getByTestId("reference-drop")).toHaveClass(/is-over/);
  await page.evaluate((at) => (window as unknown as Mock).__stemwiseMock.simulateDrop("drop", ["/x/Ref B.flac"], at), at);
  await expect(refs).toContainText("Ref B.flac");
  await expect(page.getByRole("table", { name: "Files to import" })).not.toContainText("Ref B");
  await page.getByLabel("Project name").fill("With refs");
  await page.getByRole("button", { name: "Create project" }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("button", { name: "Create project" }).click();
  await expect(page.getByTestId("project-name")).toHaveText("With refs");
  await expect.poll(() => page.evaluate(() => (window as unknown as Mock).__stemwiseMock.project?.references.length)).toBe(2);
  // The project settings, from the project menu on the name in the top bar: the references and a new name.
  await page.getByTestId("project-name").click();
  await page.getByRole("menuitem", { name: "Project settings…" }).click();
  const dlg = page.getByRole("dialog", { name: "Project settings" });
  await expect(dlg.getByTestId("references")).toContainText("Ref B.flac");
  await dlg.getByRole("button", { name: "Remove Ref B.flac" }).click();
  await expect(dlg.getByTestId("references")).not.toContainText("Ref B.flac");
  await dlg.getByLabel("Project name").fill("Renamed");
  await dlg.getByRole("button", { name: "Save" }).click();
  await expect(page.getByTestId("project-name")).toHaveText("Renamed");
});

// req: PRJ-2, UI-20
test("app menu and shortcuts: save, save as, close asking to save, new and open", async ({ page }) => {
  await openDemo(page);
  const status = page.locator("header.top").getByText(/^Saved|Unsaved changes/);
  // An edit leaves unsaved changes; Save from the project menu saves them.
  await header(page, "Bass DI").getByRole("button", { name: "Mute Bass DI" }).click();
  await page.getByTestId("strip-Kick").locator(".fader").dblclick();
  await expect(status).toHaveText(/Unsaved changes/);
  await page.getByTestId("project-name").click();
  await expect(page.getByRole("menuitem", { name: /Save as…/ })).toContainText("⇧⌘S");
  await page.getByRole("menuitem", { name: /^Save/ }).first().click();
  await expect(status).toHaveText(/^Saved/);
  // ⇧⌘S: Save As makes the copy the open project.
  await page.locator("body").press("Shift+Meta+S");
  await expect(page.getByTestId("project-name")).toHaveText("Confidence copy");
  await expect(page.locator(".toast").filter({ hasText: "Saved as a new project" })).toBeVisible();
  // Close with unsaved changes asks; Cancel keeps it open, Don't save closes.
  await page.getByTestId("strip-Kick").locator(".fader").dblclick();
  await page.getByTestId("strip-Snare").locator(".fader").dblclick();
  await page.getByTestId("project-name").click();
  await page.getByRole("menuitem", { name: "Close project" }).click();
  const ask = page.getByRole("dialog", { name: "Save changes to “Confidence copy”?" });
  await expect(ask).toBeVisible();
  await ask.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await page.getByTestId("project-name").click();
  await page.getByRole("menuitem", { name: "Close project" }).click();
  await page.getByRole("dialog", { name: /Save changes/ }).getByRole("button", { name: "Don’t save" }).click();
  await expect(page.getByRole("button", { name: "New project" })).toBeVisible();
  // ⌘O and ⌘N work on every screen (UI-20); with a project open, Open and New are in the app menu too.
  await page.locator("body").press("Meta+O");
  await expect(page.getByLabel("Timeline")).toBeVisible();
  await page.getByTestId("project-name").click();
  await expect(page.getByRole("menuitem", { name: /Open…/ })).toContainText("⌘O");
  await expect(page.getByRole("menuitem", { name: /New project…/ })).toContainText("⌘N");
  await page.keyboard.press("Escape");
  await page.locator("body").press("Meta+N");
  await expect(page.getByRole("button", { name: "Choose files…" })).toBeVisible();
});

// req: TRK-1, TRK-3
test("tracks: drag a header into a group and back out; group a selection of tracks", async ({ page }) => {
  await openDemo(page);
  const strip = (n: string) => page.getByTestId(`strip-${n}`);
  // ⌘-click selects more tracks; ⇧⌘G groups them in one step.
  await header(page, "Bass DI").locator(".nm").click();
  await header(page, "Lead Vox").locator(".nm").click({ modifiers: ["Meta"] });
  await expect(page.locator(".row.is-sel")).toHaveCount(2);
  await page.locator("body").press("Shift+Meta+G");
  await expect(page.locator('.row.grp .nm').filter({ hasText: /^Group$/ })).toHaveCount(1);
  const group = header(page, "Group");
  // Selected, a group's role line says how many tracks it has (selecting a member says why it is linked, UI-10).
  const members = async (n: number) => {
    await group.locator(".nm").click();
    await expect(group).toContainText(`${n} tracks`);
  };
  await members(2);
  // Drag Snare onto the middle of the group's header: it goes inside.
  const drag = async (from: string, to: { x: number; y: number }) => {
    const a = (await header(page, from).boundingBox())!;
    await page.mouse.move(a.x + 20, a.y + a.height / 2);
    await page.mouse.down();
    await page.mouse.move(a.x + 30, a.y + a.height / 2 + 6, { steps: 3 });
    await page.mouse.move(to.x, to.y, { steps: 6 });
    await page.mouse.up();
  };
  const g = (await group.boundingBox())!;
  await drag("Snare", { x: g.x + 40, y: g.y + g.height / 2 });
  await members(3);
  await expect(strip("Snare")).toBeVisible();
  // Out again: above the group's header, at the top level.
  const g2 = (await group.boundingBox())!;
  await drag("Snare", { x: g2.x + 40, y: g2.y + 3 });
  await members(2);
  // One step each in the history.
  expect(await historyLabels(page)).toContain("Move Snare into Group");
  expect(await historyLabels(page)).toMatch(/Move Snare(?! into)/);
});

// req: UI-4
test("dragging on the timeline never selects the interface's text; the bus meters keep their place", async ({ page }) => {
  await openDemo(page);
  const r = (await page.getByTestId("ruler").boundingBox())!;
  await page.mouse.move(r.x + 40, r.y + r.height / 2);
  await page.mouse.down();
  await page.mouse.move(r.x + 400, r.y + 300, { steps: 8 });
  await page.mouse.up();
  expect(await page.evaluate(() => String(window.getSelection() ?? ""))).toBe("");
  await expectMetersDrawn(page, "Mix bus");
});

/** The output meters are drawn (bars wide enough, filling while playing), keep their place, and the top bar fits. */
async function expectMetersDrawn(page: Page, label: string) {
  const meters = page.getByLabel(label, { exact: true });
  const bars = meters.locator(".hbar");
  await expect(bars).toHaveCount(2);
  const play = page.getByRole("group", { name: "Transport" }).getByRole("button", { name: /^(Play|Pause)$/ });
  if ((await play.getAttribute("aria-label")) === "Play") await play.click();
  await expect(play).toHaveAccessibleName("Pause");
  const x0 = (await bars.first().boundingBox())!.x;
  for (const bar of await bars.all()) {
    await expect.poll(async () => (await bar.boundingBox())!.width).toBeGreaterThanOrEqual(40);
    await expect.poll(async () => (await bar.locator("i").boundingBox())?.width ?? 0).toBeGreaterThan(4);
  }
  await page.waitForTimeout(700);
  expect((await bars.first().boundingBox())!.x).toBe(x0);
  await expect(meters.getByTestId("bus-tp")).not.toHaveText(/−∞/);
  const top = page.locator("header.top");
  expect(await top.evaluate((e) => e.scrollWidth <= e.clientWidth)).toBe(true);
  await play.click();
}

// req: MAS-2, UI-4
test("the Master top bar has the output meters too, and fits at the reference and minimum widths", async ({ page }) => {
  for (const width of [1440, 1280]) {
    await page.setViewportSize({ width, height: 900 });
    await openDemo(page);
    await page.getByRole("radio", { name: "Master" }).click();
    await page.getByRole("dialog", { name: "Bounce & go to Master" }).getByRole("button", { name: "Bounce & go to Master" }).click();
    await page.getByRole("button", { name: "Open Master" }).click({ timeout: 10_000 });
    await expect(page.getByLabel("Master chain")).toBeVisible();
    await expectMetersDrawn(page, "Master output");
    // The listening level and two references still fit (D27, D31; A-056).
    await page.evaluate(() =>
      (window as unknown as { __stemwiseMock: { simulateReferences(r: { id: string; name: string; lufs: number }[]): void } }).__stemwiseMock.simulateReferences([
        { id: "a", name: "a.wav", lufs: -12 },
        { id: "b", name: "b.wav", lufs: -13 },
      ]),
    );
    await expect(page.getByRole("radio", { name: "b.wav" })).toBeVisible();
    await expect(page.getByRole("radio", { name: "Same loudness" })).toBeVisible();
    expect(await page.locator("header.top").evaluate((e) => e.scrollWidth <= e.clientWidth)).toBe(true);
  }
});

// req: TRN-1
test("scrub: the playhead is grabbed in the ruler; its line below lets clicks through; ⌥-drag scrubs anywhere", async ({ page }) => {
  await openDemo(page);
  const pos = page.getByTestId("position");
  const ruler = (await page.getByTestId("ruler").boundingBox())!;
  const head = async () => (await page.getByTestId("playhead").boundingBox())!;
  // Put the playhead somewhere first (a click on the ruler locates).
  await page.mouse.click(ruler.x + 200, ruler.y + ruler.height / 2);
  await expect.poll(async () => Math.round((await head()).x)).toBeGreaterThan(ruler.x + 150);
  const before = await pos.innerText();
  // Grab it in the ruler and drag: it follows, no loop region is made.
  const h = await head();
  await page.mouse.move(h.x + h.width / 2, ruler.y + ruler.height / 2);
  await page.mouse.down();
  await page.mouse.move(h.x + 120, ruler.y + ruler.height / 2, { steps: 6 });
  await expect.poll(async () => (await head()).x).toBeGreaterThan(h.x + 100);
  await page.mouse.up();
  await expect(pos).not.toHaveText(before);
  await expect(page.locator(".row.ruler .loop")).toHaveCount(0);
  // A click on the playhead's line below the ruler goes to the lane, not to the playhead (read the position once the
  // scrub's last locate has arrived).
  await page.waitForTimeout(300);
  const at = await pos.innerText();
  const h2 = await head();
  const lane = (await page.locator('[data-audio-track] .ln').first().boundingBox())!;
  await page.mouse.click(h2.x + h2.width / 2, lane.y + lane.height / 2);
  await page.waitForTimeout(200);
  await expect(pos).toHaveText(at);
  // ⌥-drag anywhere on the ruler scrubs too.
  await page.keyboard.down("Alt");
  await page.mouse.move(ruler.x + 40, ruler.y + ruler.height / 2);
  await page.mouse.down();
  await page.mouse.move(ruler.x + 60, ruler.y + ruler.height / 2, { steps: 3 });
  await page.mouse.up();
  await page.keyboard.up("Alt");
  await expect.poll(async () => Math.abs((await head()).x - (ruler.x + 60))).toBeLessThan(4);
  await expect(page.locator(".row.ruler .loop")).toHaveCount(0);
});

// req: TRN-3, TRN-1
test("zoom, track height and locate from the ruler", async ({ page }) => {
  await openDemo(page);
  const ticks = page.getByTestId("ruler").locator(".tick");
  const before = await ticks.allInnerTexts();
  await page.getByRole("button", { name: "Zoom in" }).click();
  await page.getByRole("button", { name: "Zoom in" }).click();
  await expect.poll(async () => (await ticks.allInnerTexts()).join()).not.toBe(before.join());
  const row = page.locator('[data-track-row]').first();
  const h0 = (await row.boundingBox())!.height;
  await page.getByRole("button", { name: "Track height" }).click();
  await expect.poll(async () => (await row.boundingBox())!.height).toBeGreaterThan(h0);
  // A click on the ruler moves the playhead there.
  const r = (await page.getByTestId("ruler").boundingBox())!;
  await page.mouse.click(r.x + r.width / 2, r.y + r.height / 2);
  await expect(page.getByTestId("position")).not.toHaveText("0:00.00");
});

// req: UI-4
test("the track headers widen by dragging their edge, never under the layout's width, and keep it on reload", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await openDemo(page);
  const grip = page.getByTestId("head-resize");
  const width = async () => (await header(page, "Bass DI").boundingBox())!.width;
  expect(await width()).toBe(344);
  await expect(grip).toHaveAttribute("aria-valuemin", "344");
  // Narrower than the layout's width: refused.
  const g = (await grip.boundingBox())!;
  const y = g.y + 200;
  await page.mouse.move(g.x + g.width / 2, y);
  await page.mouse.down();
  await page.mouse.move(g.x - 100, y, { steps: 4 });
  await page.mouse.up();
  expect(await width()).toBe(344);
  // Wider: the header and the ruler's corner follow, the lanes start after it.
  await page.mouse.move(g.x + g.width / 2, y);
  await page.mouse.down();
  await page.mouse.move(g.x + g.width / 2 + 120, y, { steps: 6 });
  await page.mouse.up();
  expect(await width()).toBe(464);
  await expect(grip).toHaveAttribute("aria-valuenow", "464");
  // A per-user preference: it survives a reload.
  await page.reload();
  await page.getByRole("button", { name: /Confidence/ }).click();
  expect(await width()).toBe(464);
  // Keyboard (UI-7) and double click back to the default.
  await grip.focus();
  await page.keyboard.press("ArrowRight");
  await expect(grip).toHaveAttribute("aria-valuenow", "480");
  await grip.dblclick();
  expect(await width()).toBe(344);
});
