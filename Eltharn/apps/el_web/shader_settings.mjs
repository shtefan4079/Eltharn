export class ShaderSettingsEditor {
  constructor() {
    this.key = null;
    this.generation = 0;
    this.state = "empty";
    this.definitions = [];
    this.defaults = {};
    this.overrides = {};
    this.error = "";
  }

  async select(key, overrides, loadDefaults) {
    if (key === this.key) return false;
    this.key = key;
    const generation = ++this.generation;
    this.state = key === null ? "empty" : "loading";
    this.definitions = [];
    this.defaults = {};
    this.overrides = structuredClone(overrides || {});
    this.error = "";
    if (key === null) return true;
    try {
      const result = await loadDefaults();
      if (generation !== this.generation) return false;
      this.definitions = result.definitions;
      this.defaults = result.values;
      this.state = "ready";
    } catch (error) {
      if (generation !== this.generation) return false;
      this.state = "failed";
      this.error = String(error?.message || error);
    }
    return true;
  }

  value(id) {
    return Object.hasOwn(this.overrides, id) ? this.overrides[id] : this.defaults[id];
  }

  set(id, value) {
    if (this.state !== "ready" || !this.definitions.some(d => d.id === id))
      throw new Error("Unknown shader setting");
    this.overrides[id] = value;
  }

  reset() {
    this.overrides = {};
    this.error = "";
  }

  settings() {
    if (this.state !== "ready") throw new Error("Shader settings are not ready");
    const result = {};
    for (const [id, raw] of Object.entries(this.overrides)) {
      const definition = this.definitions.find(d => d.id === id);
      if (!definition) throw new Error(`Unknown shader setting: ${id}. Reset to package defaults.`);
      let value = raw;
      if (definition.type === "int" || definition.type === "float") {
        if ((typeof raw !== "string" && typeof raw !== "number") || String(raw).trim() === "")
          throw new Error(`${id}: enter a number`);
        value = Number(raw);
        if (!Number.isFinite(value) || (definition.type === "int" && !Number.isInteger(value))
            || value < definition.min || value > definition.max)
          throw new Error(`${id}: expected ${definition.type} from ${definition.min} to ${definition.max}`);
      } else if (definition.type === "bool") {
        if (typeof raw !== "boolean") throw new Error(`${id}: expected a checkbox value`);
      } else if (definition.type === "enum") {
        if (!definition.values.includes(raw)) throw new Error(`${id}: choose one of the available values`);
      } else throw new Error(`${id}: unsupported setting type`);
      result[id] = value;
    }
    return result;
  }

  async apply(validate, persist) {
    const generation = this.generation;
    const key = this.key;
    const settings = this.settings();
    await validate(settings);
    if (generation !== this.generation || key !== this.key)
      throw new Error("Shader selection changed; settings were not saved");
    await persist(settings, key);
  }
}
