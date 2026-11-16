import { describe, expect, it } from "vitest";
import { baseName, parentDir } from "./path";

// req: PRJ-2, PRJ-1
describe("paths from every platform", () => {
  it("names the last component with either separator", () => {
    expect(baseName("/Users/me/Song/Kick.wav")).toBe("Kick.wav");
    expect(baseName("C:\\Users\\me\\Song\\Kick.wav")).toBe("Kick.wav");
    expect(baseName("C:\\Users/me\\Song.stemwise\\")).toBe("Song.stemwise");
    expect(baseName("Kick.wav")).toBe("Kick.wav");
  });

  it("gives the parent folder, keeping a root", () => {
    expect(parentDir("/Users/me/Song.stemwise")).toBe("/Users/me");
    expect(parentDir("/Song.stemwise")).toBe("/");
    expect(parentDir("C:\\Users\\me\\Song.stemwise")).toBe("C:\\Users\\me");
    expect(parentDir("C:\\Song.stemwise")).toBe("C:\\");
    expect(parentDir("Song.stemwise")).toBe("");
  });
});
