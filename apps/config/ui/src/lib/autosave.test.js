import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createAutosave } from "./autosave.js";

describe("createAutosave", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("连续改动只在停顿后写一次盘", async () => {
    const write = vi.fn(async () => true);
    const a = createAutosave(write, 600);
    a.schedule();
    a.schedule();
    a.schedule();
    expect(write).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(600);
    expect(write).toHaveBeenCalledTimes(1);
  });

  it("flush 立即写盘排队中的改动并取消定时器", async () => {
    const write = vi.fn(async () => true);
    const a = createAutosave(write, 600);
    a.schedule();
    await expect(a.flush()).resolves.toBe(true);
    expect(write).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(600);
    expect(write).toHaveBeenCalledTimes(1);
  });

  it("无待存改动时 flush 视为成功且不写盘", async () => {
    const write = vi.fn(async () => true);
    const a = createAutosave(write);
    await expect(a.flush()).resolves.toBe(true);
    expect(write).not.toHaveBeenCalled();
  });

  it("flush 把写盘失败如实报告给调用方", async () => {
    const a = createAutosave(async () => false);
    a.schedule();
    await expect(a.flush()).resolves.toBe(false);
  });

  it("写盘途中排队的改动不丢：上一笔完成后补写", async () => {
    let release;
    const gate = new Promise((r) => (release = r));
    const write = vi.fn(async () => {
      if (write.mock.calls.length === 1) await gate;
      return true;
    });
    const a = createAutosave(write, 100);

    a.schedule();
    await vi.advanceTimersByTimeAsync(100);
    expect(write).toHaveBeenCalledTimes(1); // 第一笔卡在写盘中

    a.schedule(); // 写盘途中又有改动
    await vi.advanceTimersByTimeAsync(100);
    expect(write).toHaveBeenCalledTimes(1); // 不并发写

    release();
    await vi.advanceTimersByTimeAsync(0);
    expect(write).toHaveBeenCalledTimes(2); // 第一笔完成后补写
  });

  it("补写完成后遗留的定时器不再触发多余写盘", async () => {
    let release;
    const gate = new Promise((r) => (release = r));
    const write = vi.fn(async () => {
      if (write.mock.calls.length === 1) await gate;
      return true;
    });
    const a = createAutosave(write, 100);

    a.schedule();
    await vi.advanceTimersByTimeAsync(100); // 第一笔写盘中
    a.schedule();
    await vi.advanceTimersByTimeAsync(100); // 定时器到点时第一笔未完 → 挂上补写
    a.schedule(); // 又一次改动：仅换上新定时器，补写仍挂在第一笔上

    release();
    await vi.advanceTimersByTimeAsync(0); // 第一笔完成 → 补写第二笔（捕获全部改动）
    expect(write).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(100); // 遗留定时器到点
    expect(write).toHaveBeenCalledTimes(2); // 改动已全部落盘，不再写
  });

  it("flush 等到全部改动落盘才返回", async () => {
    let release;
    const gate = new Promise((r) => (release = r));
    const write = vi.fn(async () => {
      if (write.mock.calls.length === 1) await gate;
      return true;
    });
    const a = createAutosave(write, 100);

    a.schedule();
    await vi.advanceTimersByTimeAsync(100); // 第一笔写盘中
    a.schedule(); // 途中改动

    const flushed = a.flush();
    release();
    await expect(flushed).resolves.toBe(true);
    expect(write).toHaveBeenCalledTimes(2);
    expect(a.dirty).toBe(false);
  });

  it("dirty 反映排队与写盘中的状态", async () => {
    const a = createAutosave(async () => true);
    expect(a.dirty).toBe(false);
    a.schedule();
    expect(a.dirty).toBe(true);
    await a.flush();
    expect(a.dirty).toBe(false);
  });

  it("write 抛异常按失败处理，调度器之后仍可用", async () => {
    let fail = true;
    const a = createAutosave(async () => {
      if (fail) throw new Error("disk full");
      return true;
    });
    a.schedule();
    await expect(a.flush()).resolves.toBe(false);

    fail = false;
    a.schedule();
    await expect(a.flush()).resolves.toBe(true);
  });
});
