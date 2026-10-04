import type { HardwareStatus } from "./api";

// Serialize probes and let card reads take exclusive ownership of the adaptor.
export function createAdaptorMonitor(
  probe: () => Promise<HardwareStatus>,
  update: (status: HardwareStatus) => void,
  checking: (value: boolean) => void,
  readOnStartup: () => void = () => undefined,
) {
  let startup = true;
  let stopped = false;
  let paused = false;
  let pending: Promise<void> | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function check(): Promise<void> {
    if (stopped || paused) return Promise.resolve();
    if (pending) return pending;
    clearTimeout(timer);
    checking(true);
    pending = (async () => {
      try {
        const status = await probe();
        if (!stopped && !paused) {
          update(status);
          const shouldRead = startup && status.state === "adaptor";
          startup = false;
          if (shouldRead) readOnStartup();
        }
      } catch (err) {
        startup = false;
        if (!stopped && !paused) update({
          state: "error",
          message: err instanceof Error ? err.message : String(err),
          identity: null,
          frame: 0,
          total: 1024,
        });
      } finally {
        pending = undefined;
        if (!stopped) {
          checking(false);
          if (!paused) timer = setTimeout(() => void check(), 2000);
        }
      }
    })();
    return pending;
  }

  return {
    check,
    async pause() {
      paused = true;
      clearTimeout(timer);
      await pending;
    },
    resume() {
      paused = false;
      void check();
    },
    stop() {
      stopped = true;
      clearTimeout(timer);
    },
  };
}
