/** Only the newest local lookup may publish a value, failure or pending state.
 * Invalidating an owner also prevents late callbacks after unmount or mutation.
 * This does not retry a failed read or cancel its underlying native operation. */
export class LatestLocalRead {
  private version = 0;
  capture(): number { return this.version; }
  owns(version: number): boolean { return version === this.version; }
  invalidate(): void { ++this.version; }

  async run<T>(load: () => Promise<T>, callbacks: {
    value: (value: T) => void; error: (error: unknown) => void; pending: (pending: boolean) => void;
  }): Promise<void> {
    const version = ++this.version;
    callbacks.pending(true);
    try {
      const value = await load();
      if (this.owns(version)) callbacks.value(value);
    } catch (error) {
      if (this.owns(version)) callbacks.error(error);
    } finally {
      if (this.owns(version)) callbacks.pending(false);
    }
  }
}
