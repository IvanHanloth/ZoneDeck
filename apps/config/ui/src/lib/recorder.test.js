import { describe, expect, it } from "vitest";
import { initRecorder, stepRecorder } from "./recorder.js";

/** 依次喂入若干快照，返回最终状态。 */
function play(snapshots, draft = "") {
  return snapshots.reduce(stepRecorder, initRecorder(draft));
}

/** 快照速写：`down("Ctrl", ["Q"])` = 此刻按住 Ctrl 与 Q，本次是按下。 */
const down = (modifiers, keys = []) => ({ modifiers, keys, down: true, unsupported: false });
const up = (modifiers, keys = []) => ({ modifiers, keys, down: false, unsupported: false });

describe("stepRecorder", () => {
  it("录一个普通组合，松开主键即定稿", () => {
    const s = play([down("Ctrl"), down("Ctrl", ["Q"]), up("Ctrl"), up("")]);
    expect(s.draft).toBe("Ctrl+Q");
  });

  it("先松修饰键再松主键，结果一样", () => {
    const s = play([down("Ctrl"), down("Ctrl", ["Q"]), up("", ["Q"]), up("")]);
    expect(s.draft).toBe("Ctrl+Q");
  });

  it("多主键要按齐才定稿", () => {
    const s = play([down("", ["Q"]), down("", ["Q", "W"]), up("", ["W"]), up("")]);
    expect(s.draft).toBe("Q+W");
  });

  it("纯修饰键在全松开时定稿", () => {
    const s = play([down("Ctrl"), down("Ctrl+Shift"), up("Ctrl"), up("")]);
    expect(s.draft).toBe("Ctrl+Shift");
  });

  it("单个主键裸绑", () => {
    const s = play([down("", ["F5"]), up("")]);
    expect(s.draft).toBe("F5");
  });

  // 回归：修饰键不松手接着录第二个组合，曾经被 committed 闸门整个挡掉。
  it("不松开修饰键也能接着录下一个组合", () => {
    const s = play([
      down("Ctrl"),
      down("Ctrl", ["Q"]),
      up("Ctrl"),
      down("Ctrl", ["W"]),
      up("Ctrl"),
      up(""),
    ]);
    expect(s.draft).toBe("Ctrl+W");
  });

  it("连录三次都跟得上", () => {
    const s = play([
      down("Ctrl"),
      down("Ctrl", ["Q"]),
      up("Ctrl"),
      down("Ctrl", ["W"]),
      up("Ctrl"),
      down("Ctrl", ["E"]),
      up("Ctrl"),
      up(""),
    ]);
    expect(s.draft).toBe("Ctrl+E");
  });

  // 回归：长按修饰键会持续送出按下事件，不能当成新一轮，否则组合被覆盖成裸修饰键。
  it("定稿后修饰键的长按重复不覆盖已录到的组合", () => {
    const s = play([
      down("Ctrl"),
      down("Ctrl", ["Q"]),
      up("Ctrl"),
      down("Ctrl"),
      down("Ctrl"),
      up(""),
    ]);
    expect(s.draft).toBe("Ctrl+Q");
  });

  it("按住期间的长按重复不影响录制", () => {
    const s = play([
      down("Ctrl"),
      down("Ctrl"),
      down("Ctrl", ["Q"]),
      down("Ctrl", ["Q"]),
      up("Ctrl"),
      up(""),
    ]);
    expect(s.draft).toBe("Ctrl+Q");
  });

  it("全松开后 committed 复位，舞台回落到 draft", () => {
    const s = play([down("Ctrl"), down("Ctrl", ["Q"]), up("Ctrl"), up("")]);
    expect(s.committed).toBe(false);
    expect(s.live).toEqual({ modifiers: "", keys: [] });
    expect(s.peak).toEqual({ modifiers: "", keys: [] });
  });

  it("松手途中冻住已录到的组合，不跟着掉键", () => {
    const s = play([down("Ctrl"), down("Ctrl", ["Q"]), up("Ctrl")]);
    expect(s.committed).toBe(true);
    expect(s.draft).toBe("Ctrl+Q");
  });

  it("不支持的键只置提示，不动已录到的组合", () => {
    const s = play(
      [{ modifiers: "Ctrl", keys: [], down: true, unsupported: true }],
      "Ctrl+Q",
    );
    expect(s.unsupported).toBe(true);
    expect(s.draft).toBe("Ctrl+Q");
  });

  it("录到能用的键后提示消掉", () => {
    const s = play([
      { modifiers: "", keys: [], down: true, unsupported: true },
      down("Ctrl", ["Q"]),
    ]);
    expect(s.unsupported).toBe(false);
  });

  it("空快照不会凭空定稿", () => {
    const s = play([up("")], "Ctrl+Q");
    expect(s.draft).toBe("Ctrl+Q");
  });
});
