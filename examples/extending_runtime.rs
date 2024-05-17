///
/// This example demonstrates extending Runtime to inline your own extensions and modules
/// as well as enforce values for the Runtime's options
///
/// This example creates a runtime which will timeout after 0.5s, imports an exension,
/// And ensures that a preset module is always available for import.
///
/// Extensions like the one being used (see examples/ext/example_extension.rs)
/// allow you to call rust code from within JS
///
/// Extensions consist of a set of #[op2] functions, an extension! macro,
/// and one or more optional JS modules.
///
use rustyscript::{
    module, serde_json, Error, Module, ModuleHandle, Runtime, RuntimeOptions, StaticModule,
};
use std::time::Duration;

// See ext/example_extension for a demonstration
// of creating a deno_core extension for the runtime
mod ext;
use ext::example_extension;

// A module that will always be loaded into the custom runtime
const MY_MODULE: StaticModule = module!(
    "my_module.js",
    "export function importantFunction() {
        return 42;
    }"
);

/// A runtime which will timeout after 0.5s, imports an exension,
/// And ensures that a preset module is always available for import.
pub struct MyRuntime(Runtime);
impl MyRuntime {
    /// Create a new instance of the runtime
    pub async fn new() -> Result<Self, Error> {
        let mut runtime = Self(Runtime::new(RuntimeOptions {
            extensions: vec![example_extension::example_extension::init_ops_and_esm()],
            timeout: Duration::from_millis(500),
            ..Default::default()
        })?);
        runtime.load_module(&MY_MODULE.to_module()).await?;

        Ok(runtime)
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the javascript function to call.
    pub async fn call_function<T>(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
        args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.0.call_function(module_context, name, args).await
    }

    /// Get a value from a runtime instance
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the value to find
    pub async fn get_value<T>(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.0.get_value(module_context, name).await
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    pub async fn load_module(&mut self, module: &Module) -> Result<ModuleHandle, Error> {
        self.0.load_module(module).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        import * as myModule from './my_module.js';
        export const value = myModule.importantFunction();
        ",
    );

    let mut runtime = MyRuntime::new().await?;
    let module_context = runtime.load_module(&module).await?;
    let value: i32 = runtime.get_value(&module_context, "value").await?;
    assert_eq!(42, value);

    Ok(())
}
