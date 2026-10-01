// Cache successful immutable reads and share concurrent requests. Failed reads
// are removed so a transient network problem can be retried.
export class ResourceCache {
  constructor(limit = 64, dispose = () => {}) { this.limit = limit; this.dispose = dispose; this.values = new Map(); }
  has(key) { return this.values.has(key); }
  get(key, load) {
    if (this.values.has(key)) {
      const value = this.values.get(key);
      this.values.delete(key); this.values.set(key, value);
      return value;
    }
    const value = Promise.resolve().then(load);
    this.values.set(key, value);
    value.catch(() => { if (this.values.get(key) === value) this.values.delete(key); });
    while (this.values.size > this.limit) {
      const oldest = this.values.keys().next().value, removed = this.values.get(oldest);
      this.values.delete(oldest); removed.then(this.dispose, () => {});
    }
    return value;
  }
}
