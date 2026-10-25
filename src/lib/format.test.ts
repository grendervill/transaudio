import { describe, expect, it } from "vitest";

import { formatBytes, formatDuration, formatTimecode } from "./format";

describe("format helpers", () => {
  it("formats byte sizes for Russian UI", () => {
    expect(formatBytes(0)).toBe("0 Б");
    expect(formatBytes(512)).toBe("512 Б");
    expect(formatBytes(1536)).toBe("1,5 КБ");
    expect(formatBytes(38_600_000)).toMatch(/^38,6 МБ$/);
    expect(formatBytes(1_200_000_000)).toMatch(/^1,2 ГБ$/);
  });

  it("formats durations for queue rows and progress", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(62)).toBe("1:02");
    expect(formatDuration(3723)).toBe("1:02:03");
  });

  it("formats transcript timecodes", () => {
    expect(formatTimecode(0)).toBe("00:00");
    expect(formatTimecode(62)).toBe("01:02");
    expect(formatTimecode(3723)).toBe("01:02:03");
  });
});
