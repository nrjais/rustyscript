// Set up this extension
globalThis.js_playground = {
    'register_entrypoint': (f) => Deno.core.ops.op_register_entrypoint(f),
    'bail': (msg) => { throw new Error(msg) }
};