import { expect, type Locator } from "@playwright/test";

/**
 * Every value of a `.mtable` sits under its header: the cells are the grid's own children (so the table's rules
 * reach them), right-aligned columns end where their header ends and the others start where it starts.
 * Rows spanning the whole table (notes) are left out.
 */
export async function expectColumnsAligned(tables: Locator) {
  const n = await tables.count();
  expect(n).toBeGreaterThan(0);
  for (let i = 0; i < n; i++) {
    const off = await tables.nth(i).evaluate((t) => {
      const text = (el: Element) => {
        const r = document.createRange();
        r.selectNodeContents(el);
        return r.getBoundingClientRect();
      };
      const kids = [...t.children] as HTMLElement[];
      const heads = kids.filter((k) => k.classList.contains("th"));
      const cells = kids.filter((k) => !k.classList.contains("th") && !k.style.gridColumn);
      const bad: string[] = [];
      if (kids.some((k) => getComputedStyle(k).display === "contents")) bad.push("a row wrapper hides the cells from the table's rules");
      cells.forEach((c, j) => {
        const h = heads[j % heads.length];
        if (!c.textContent?.trim()) return;
        const right = h.classList.contains("r");
        const d = right ? text(c).right - text(h).right : text(c).left - text(h).left;
        if (Math.abs(d) > 1) bad.push(`"${c.textContent}" is ${d.toFixed(1)} px off "${h.textContent}"`);
      });
      return bad;
    });
    expect(off, `table ${i}`).toEqual([]);
  }
}
