const ObjectProperties = {
  nonEnumerable: { writable: true, enumerable: false, configurable: true },
  readOnly: { writable: true, enumerable: false, configurable: true },
  writeable: { writable: true, enumerable: true, configurable: true },
  getterOnly: { enumerable: true, configurable: true },

  apply: (value, type) => {
    return {
      value: value,
      ...ObjectProperties[type],
    };
  },
};

const nonEnumerable = (value) => ObjectProperties.apply(value, nonEnumerable);
const readOnly = (value) => ObjectProperties.apply(value, readOnly);
const writeable = (value) => ObjectProperties.apply(value, writeable);
const getterOnly = (getter) => {
  return {
    get: getter,
    set() {},
    ...ObjectProperties.getterOnly,
  };
};

const applyToGlobal = (properties) =>
  Object.defineProperties(globalThis, properties);

globalThis.rustyscript = {
  register_entrypoint: (f) => Deno.core.ops.op_register_entrypoint(f),
};

Object.freeze(globalThis.rustyscript);

export { nonEnumerable, readOnly, writeable, getterOnly, applyToGlobal };
