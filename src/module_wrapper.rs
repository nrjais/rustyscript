use deno_core::{serde_json, v8::GetPropertyNamesArgs};

use crate::{Error, JsFunction, Module, ModuleHandle, Runtime, RuntimeOptions};

/// A wrapper type representing a runtime instance loaded with a single module
pub struct ModuleWrapper {
    module_context: ModuleHandle,
    runtime: Runtime,
}

impl ModuleWrapper {
    /// Creates a new `ModuleWrapper` from a given module and runtime options.
    ///
    /// # Arguments
    ///
    /// * `module` - A reference to the module to load.
    /// * `options` - The runtime options for the module.
    ///
    /// # Returns
    ///
    /// A `Result` containing `Self` on success or an `Error` on failure.
    pub fn new_from_module(module: &Module, options: RuntimeOptions) -> Result<Self, Error> {
        let mut runtime = Runtime::new(options)?;
        let module_context = runtime.load_module(module)?;
        Ok(Self {
            module_context,
            runtime,
        })
    }

    /// Creates a new `ModuleWrapper` from a file path and runtime options.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the module file.
    /// * `options` - The runtime options for the module.
    ///
    /// # Returns
    ///
    /// A `Result` containing `Self` on success or an `Error` on failure.
    pub fn new_from_file(path: &str, options: RuntimeOptions) -> Result<Self, Error> {
        let module = Module::load(path)?;
        Self::new_from_module(&module, options)
    }

    /// Returns a reference to the module context.
    pub fn get_module_context(&self) -> &ModuleHandle {
        &self.module_context
    }

    /// Returns a mutable reference to the underlying runtime.
    pub fn get_runtime(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// Retrieves a value from the module by name and deserializes it.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the value to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value of type `T` on success or an `Error` on failure.
    pub fn get<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime.get_value(&self.module_context, name)
    }

    /// Checks if a value in the module with the given name is callable as a JavaScript function.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the value to check for callability.
    ///
    /// # Returns
    ///
    /// `true` if the value is callable as a JavaScript function, `false` otherwise.
    pub fn is_callable(&mut self, name: &str) -> bool {
        let test = self.get::<JsFunction>(name);
        println!("{:?}", test);
        test.is_ok()
    }

    /// Calls a function in the module with the given name and arguments and deserializes the result.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    pub fn call<T>(&mut self, name: &str, args: &[serde_json::Value]) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime.call_function(&self.module_context, name, args)
    }

    /// Calls a function using the module's runtime that was previously stored
    /// as a JsFunction object
    ///
    /// # Arguments
    ///
    /// * `function` - The JsFunction to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    pub fn call_stored<T>(
        &mut self,
        function: &JsFunction,
        args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime
            .call_stored_function(&self.module_context, function, args)
    }

    /// Retrieves the names of the module's exports.
    ///
    /// # Returns
    ///
    /// A `Vec` of `String` containing the names of the keys.
    pub fn keys(&mut self) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        if let Ok(namespace) = self
            .runtime
            .deno_runtime()
            .get_module_namespace(self.module_context.id())
        {
            let mut scope = self.runtime.deno_runtime().handle_scope();
            let global = namespace.open(&mut scope);
            if let Some(keys_obj) =
                global.get_property_names(&mut scope, GetPropertyNamesArgs::default())
            {
                for i in 0..keys_obj.length() {
                    if let Ok(key_index) = deno_core::serde_v8::to_v8(&mut scope, i) {
                        if let Some(key_name_v8) = keys_obj.get(&mut scope, key_index) {
                            let name = key_name_v8.to_rust_string_lossy(&mut scope);
                            keys.push(name)
                        }
                    }
                }
            }
        }

        keys
    }
}

#[cfg(test)]
mod test_runtime {
    use super::*;
    use crate::json_args;

    #[test]
    fn test_call() {
        let module = Module::new(
            "test.js",
            "
            console.log('test');
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");
        let value: usize = module
            .call("func", json_args!())
            .expect("Could not call function");
        assert_eq!(4, value);
    }

    #[test]
    fn test_get() {
        let module = Module::new(
            "test.js",
            "
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");
        let value: usize = module.get("value").expect("Could not get value");
        assert_eq!(3, value);
    }

    #[test]
    fn test_callable() {
        let module = Module::new(
            "test.js",
            "
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");

        assert!(module.is_callable("func"));
        assert!(!module.is_callable("value"));
    }

    #[test]
    fn test_keys() {
        let module = Module::new(
            "test.js",
            "
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");
        let mut keys = module.keys();
        assert_eq!(2, keys.len());
        assert_eq!("value", keys.pop().unwrap());
        assert_eq!("func", keys.pop().unwrap());
    }
}
