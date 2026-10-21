const MAX_BYTES = 8 * 1024 * 1024;
const MAX_DEPTH = 64;
const MAX_NODES = 1048576;
const encoder = new TextEncoder();

export function stringifyJson(value) {
  const pieces = [], ancestors = new Set();
  let bytes = 0, nodes = 0;
  const append = text => {
    bytes += encoder.encode(text).length;
    if (bytes > MAX_BYTES) throw new Error('JSON exceeds 8 MiB');
    pieces.push(text);
  };
  const string = value => {
    if (value.length > MAX_BYTES) throw new Error('JSON string exceeds limit');
    append(JSON.stringify(value));
  };
  const visit = (value, depth) => {
    if (++nodes > MAX_NODES || depth > MAX_DEPTH) throw new Error('JSON depth/element limit');
    if (value === null) { append('null'); return; }
    switch (typeof value) {
      case 'boolean': append(value ? 'true' : 'false'); return;
      case 'string': string(value); return;
      case 'number':
        if (!Number.isFinite(value)) throw new Error('Nonfinite JSON number');
        append(Object.is(value, -0) ? '-0.0'
          : Number.isInteger(value) && !Number.isSafeInteger(value) ? value.toExponential()
          : value.toString());
        return;
      case 'object': break;
      default: throw new Error('Unsupported JSON value');
    }
    if (ancestors.has(value)) throw new Error('Cyclic JSON value');
    const array = Array.isArray(value);
    if (!array && ![Object.prototype, null].includes(Object.getPrototypeOf(value))) {
      throw new Error('Non-JSON object');
    }
    ancestors.add(value);
    append(array ? '[' : '{');
    const keys = array ? null : Object.keys(value);
    const count = array ? value.length : keys.length;
    if (count > MAX_NODES) throw new Error('JSON element limit');
    let separator = false;
    for (let index = 0; index < count; index++) {
      const key = array ? String(index) : keys[index];
      const property = Object.getOwnPropertyDescriptor(value, key);
      if (property && !Object.hasOwn(property, 'value')) throw new Error('JSON accessor rejected');
      const child = property?.value;
      if (!array && child === undefined) continue;
      if (separator) append(',');
      separator = true;
      if (!array) { string(key); append(':'); }
      visit(child === undefined ? null : child, depth + 1);
    }
    append(array ? ']' : '}');
    ancestors.delete(value);
  };
  visit(value, 0);
  return pieces.join('');
}
