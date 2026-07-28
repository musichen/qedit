type RegistryEntry<T> = {
  id: string;
  factory: () => T;
};

class Registry<T> {
  private entries = new Map<string, RegistryEntry<T>>();

  register(id: string, factory: () => T): void {
    if (this.entries.has(id)) {
      throw new Error(`Registry entry "${id}" already exists`);
    }

    this.entries.set(id, { id, factory });
  }

  get(id: string): T | undefined {
    const entry = this.entries.get(id);

    return entry?.factory();
  }

  getAll(): T[] {
    return Array.from(this.entries.values()).map((entry) => entry.factory());
  }
}

export function createRegistry<T>() {
  return new Registry<T>();
}
