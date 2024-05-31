use deno_core::{extension, Extension};

extension!(
    rustyscript,
    esm_entry_point = "ext:rustyscript/rustyscript.js",
    esm = [ dir "src/ext/rustyscript", "rustyscript.js" ],
);

pub fn extensions() -> Vec<Extension> {
    vec![rustyscript::init_ops_and_esm()]
}
