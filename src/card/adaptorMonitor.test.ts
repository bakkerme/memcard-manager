import { afterEach, expect, it, vi } from "vitest";
import { createAdaptorMonitor } from "./adaptorMonitor";
import type { HardwareStatus } from "./api";

const absent: HardwareStatus = { state: "searching", message: "Not found", identity: null, frame: 0, total: 1024 };
const present = { ...absent, state: "adaptor" };
afterEach(() => vi.useRealTimers());

it("detects attachment after an empty startup and later removal", async () => {
  vi.useFakeTimers();
  const probe = vi.fn().mockResolvedValueOnce(absent).mockResolvedValueOnce(present).mockResolvedValue(absent);
  const update = vi.fn();
  const monitor = createAdaptorMonitor(probe, update, vi.fn());
  await monitor.check();
  expect(update).toHaveBeenLastCalledWith(absent);
  await vi.advanceTimersByTimeAsync(2000);
  expect(update).toHaveBeenLastCalledWith(present);
  await vi.advanceTimersByTimeAsync(2000);
  expect(update).toHaveBeenLastCalledWith(absent);
  monitor.stop();
});

it("allows an immediate manual retry and recovers after probe errors", async () => {
  vi.useFakeTimers();
  const probe = vi.fn().mockRejectedValueOnce(new Error("USB unavailable")).mockResolvedValue(present);
  const update = vi.fn();
  const monitor = createAdaptorMonitor(probe, update, vi.fn());
  await monitor.check();
  expect(update).toHaveBeenLastCalledWith(expect.objectContaining({ state: "error", message: "USB unavailable" }));
  await monitor.check();
  expect(update).toHaveBeenLastCalledWith(present);
  monitor.stop();
});

it("coalesces checks and waits for an outstanding probe before reading", async () => {
  vi.useFakeTimers();
  let resolve!: (status: HardwareStatus) => void;
  const probe = vi.fn().mockImplementationOnce(() => new Promise<HardwareStatus>((r) => { resolve = r; })).mockResolvedValue(present);
  const update = vi.fn();
  const monitor = createAdaptorMonitor(probe, update, vi.fn());
  const first = monitor.check();
  expect(monitor.check()).toBe(first);
  const paused = vi.fn();
  const pause = monitor.pause().then(paused);
  await vi.advanceTimersByTimeAsync(6000);
  expect(probe).toHaveBeenCalledTimes(1);
  expect(paused).not.toHaveBeenCalled();
  resolve(absent);
  await pause;
  expect(update).not.toHaveBeenCalled();
  await monitor.check();
  expect(probe).toHaveBeenCalledTimes(1);
  monitor.resume();
  await vi.advanceTimersByTimeAsync(0);
  expect(update).toHaveBeenLastCalledWith(present);
  monitor.stop();
});

it("ignores late results and clears timers after cleanup", async () => {
  vi.useFakeTimers();
  let resolve!: (status: HardwareStatus) => void;
  const probe = vi.fn(() => new Promise<HardwareStatus>((r) => { resolve = r; }));
  const update = vi.fn();
  const monitor = createAdaptorMonitor(probe, update, vi.fn());
  const pending = monitor.check();
  monitor.stop();
  resolve(present);
  await pending;
  await vi.advanceTimersByTimeAsync(10000);
  expect(update).not.toHaveBeenCalled();
  expect(probe).toHaveBeenCalledTimes(1);
});

it("reads an adaptor present at startup once, even after a failed read and resume", async () => {
  vi.useFakeTimers();
  const read = vi.fn();
  const monitor = createAdaptorMonitor(vi.fn().mockResolvedValue(present), vi.fn(), vi.fn(), read);
  await monitor.check();
  expect(read).toHaveBeenCalledTimes(1);
  await monitor.pause();
  monitor.resume();
  await vi.advanceTimersByTimeAsync(6000);
  expect(read).toHaveBeenCalledTimes(1);
  monitor.stop();
});

it("does not auto-read a later attachment or an unmounted startup", async () => {
  vi.useFakeTimers();
  const read = vi.fn();
  const monitor = createAdaptorMonitor(vi.fn().mockResolvedValueOnce(absent).mockResolvedValue(present), vi.fn(), vi.fn(), read);
  await monitor.check();
  await vi.advanceTimersByTimeAsync(2000);
  expect(read).not.toHaveBeenCalled();
  monitor.stop();
  const stopped = createAdaptorMonitor(vi.fn().mockResolvedValue(present), vi.fn(), vi.fn(), read);
  const pending = stopped.check();
  stopped.stop();
  await pending;
  expect(read).not.toHaveBeenCalled();
});

it("lets the startup read pause the probe without deadlocking", async () => {
  vi.useFakeTimers();
  let pause: Promise<void> | undefined;
  const monitor = createAdaptorMonitor(vi.fn().mockResolvedValue(present), vi.fn(), vi.fn(), () => {
    pause = monitor.pause();
  });
  await monitor.check();
  await pause;
  expect(vi.getTimerCount()).toBe(0);
  monitor.stop();
});
