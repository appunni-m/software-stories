/** A cancelled waiting question still releases its slot without starting inference. */
export function createQuestionQueue() {
  let tail = Promise.resolve();
  return async (): Promise<() => void> => {
    const previous = tail;
    let release!: () => void;
    tail = new Promise<void>((resolve) => { release = resolve; });
    await previous;
    return release;
  };
}
