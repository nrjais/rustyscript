use crate::{
    inner_runtime::{InnerRuntime, InnerRuntimeOptions},
    Error, FunctionArguments, JsFunction, Module, ModuleHandle,
};
use deno_core::serde_json;

/// Represents the set of options accepted by the runtime constructor
pub type RuntimeOptions = InnerRuntimeOptions;

/// For functions returning nothing
pub type Undefined = serde_json::Value;

/// Represents a configured runtime ready to run modules
pub struct Runtime(InnerRuntime);

impl Runtime {
    /// The lack of any arguments - used to simplify calling functions
    /// Prevents you from needing to specify the type using ::<serde_json::Value>
    pub const EMPTY_ARGS: &'static FunctionArguments = &[];

    /// Creates a new instance of the runtime with the provided options.
    ///
    /// # Arguments
    /// * `options` - A `RuntimeOptions` struct that specifies the configuration options for the runtime.
    ///
    /// # Returns
    /// A `Result` containing either the initialized runtime instance on success (`Ok`) or an error on failure (`Err`).
    ///
    /// # Example
    /// ```rust
    /// use rustyscript::{ json_args, Runtime, RuntimeOptions, Module };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// # tokio_test::block_on(async {
    /// // Creates a runtime that will attempt to run function load() on start
    /// // And which will time-out after 50ms
    /// let mut runtime = Runtime::new(RuntimeOptions {
    ///     default_entrypoint: Some("load".to_string()),
    ///     timeout: Duration::from_millis(50),
    ///     ..Default::default()
    /// })?;
    ///
    /// let module = Module::new("test.js", "
    ///     export const load = () => {
    ///         return 'Hello World!';
    ///     }
    /// ");
    ///
    /// let module_handle = runtime.load_module(&module).await?;
    /// let value: String = runtime.call_entrypoint(&module_handle, json_args!()).await?;
    /// assert_eq!("Hello World!", value);
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    ///
    pub fn new(options: RuntimeOptions) -> Result<Self, Error> {
        Ok(Self(InnerRuntime::new(options)))
    }

    /// Access the underlying deno runtime instance directly
    pub fn deno_runtime(&mut self) -> &mut deno_core::JsRuntime {
        self.0.deno_runtime()
    }

    /// Access the options used to create this runtime
    pub fn options(&self) -> &RuntimeOptions {
        &self.0.options
    }

    /// Encode an argument as a json value for use as a function argument
    /// ```rust
    /// use rustyscript::{ Runtime, RuntimeOptions, Module };
    /// use serde::Serialize;
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// # tokio_test::block_on(async {
    /// let module = Module::new("test.js", "
    ///     function load(obj) {
    ///         console.log(`Hello world: a=${obj.a}, b=${obj.b}`);
    ///     }
    ///     rustyscript.register_entrypoint(load);
    /// ");
    ///
    /// #[derive(Serialize)]
    /// struct MyStruct {a: usize, b: usize}
    ///
    /// Runtime::execute_module(
    ///     &module, vec![],
    ///     Default::default(),
    ///     &[
    ///         Runtime::arg(MyStruct{a: 1, b: 2})?,
    ///     ]
    /// ).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub fn arg<A>(value: A) -> Result<serde_json::Value, Error>
    where
        A: serde::Serialize,
    {
        Ok(serde_json::to_value(value)?)
    }

    /// Encode a primitive as a json value for use as a function argument
    /// Only for types with `Into<Value>`. For other types, use `Runtime::arg`
    /// ```rust
    /// use rustyscript::{ Runtime, RuntimeOptions, Module };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// # tokio_test::block_on(async {
    /// let module = Module::new("test.js", "
    ///     function load(a, b) {
    ///         console.log(`Hello world: a=${a}, b=${b}`);
    ///     }
    ///     rustyscript.register_entrypoint(load);
    /// ");
    ///
    /// Runtime::execute_module(
    ///     &module, vec![],
    ///     Default::default(),
    ///     &[
    ///         Runtime::into_arg("test"),
    ///         Runtime::into_arg(5),
    ///     ]
    /// ).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub fn into_arg<A>(value: A) -> serde_json::Value
    where
        serde_json::Value: From<A>,
    {
        serde_json::Value::from(value)
    }

    /// Remove and return a value from the state, if one exists
    /// ```rust
    /// use rustyscript::{ Runtime };
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// runtime.put("test".to_string())?;
    /// let value: String = runtime.take().unwrap();
    /// assert_eq!(value, "test");
    /// # Ok(())
    /// # }
    /// ```
    pub fn take<T>(&mut self) -> Option<T>
    where
        T: 'static,
    {
        self.0.take()
    }

    /// Add a value to the state
    /// Only one value of each type is stored - additional calls to put overwrite the
    /// old value
    /// ```rust
    /// use rustyscript::{ Runtime };
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// runtime.put("test".to_string())?;
    /// let value: String = runtime.take().unwrap();
    /// assert_eq!(value, "test");
    /// # Ok(())
    /// # }
    /// ```
    pub fn put<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: 'static,
    {
        self.0.put(value)
    }

    /// Evaluate a piece of non-ECMAScript-module JavaScript code
    /// The expression is evaluated in the global context, so changes persist
    ///
    /// # Arguments
    /// * `expr` - A string representing the JavaScript expression to evaluate
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the expression (`T`)
    /// or an error (`Error`) if the expression cannot be evaluated or if the
    /// result cannot be deserialized.
    ///
    /// # Example
    /// ```rust
    /// use rustyscript::{ Runtime, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let value:
    ///    usize = runtime.eval("2 + 2")?;
    /// assert_eq!(4, value);
    /// # Ok(())
    /// # }
    /// ```
    pub fn eval<T>(&mut self, expr: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.0.eval(expr)
    }

    /// Calls a stored javascript function and deserializes its return value.
    ///
    /// # Arguments
    /// * `function` - A The function object
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    pub async fn call_stored_function<'a, T>(
        &'a mut self,
        module_context: &ModuleHandle,
        function: &JsFunction<'a>,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.0
            .call_stored_function(module_context, function, args)
            .await
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the javascript function to call.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::{ json_args, Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// # tokio_test::block_on(async {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "export function f() { return 2; };");
    /// let module = runtime.load_module(&module).await?;
    /// let value: usize = runtime.call_function(&module, "f", json_args!()).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub async fn call_function<T>(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
        args: &FunctionArguments,
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
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the
    /// value cannot be found, if there are issues with, or if the result cannot be
    ///  deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::{ Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// # tokio_test::block_on(async {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "globalThis.my_value = 2;");
    /// let module = runtime.load_module(&module).await?;
    /// let value: usize = runtime.get_value(&module, "my_value").await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
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
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// module, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use rustyscript::{Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// # tokio_test::block_on(async {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "rustyscript.register_entrypoint(() => 'test')");
    /// runtime.load_module(&module).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub async fn load_module(&mut self, module: &Module) -> Result<ModuleHandle, Error> {
        self.0.load_modules(None, vec![module]).await
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions.
    ///
    /// This will load 'module' as the main module, and the others as side-modules.
    /// Only one main module can be loaded per runtime
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// module, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use rustyscript::{Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// # tokio_test::block_on(async {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "rustyscript.register_entrypoint(() => 'test')");
    /// runtime.load_modules(&module, vec![]).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub async fn load_modules(
        &mut self,
        module: &Module,
        side_modules: Vec<&Module>,
    ) -> Result<ModuleHandle, Error> {
        self.0.load_modules(Some(module), side_modules).await
    }

    /// Executes the entrypoint function of a module within the Deno runtime.
    ///
    /// # Arguments
    /// * `module_context` - A handle returned by loading a module into the runtime
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the entrypoint execution (`T`)
    /// if successful, or an error (`Error`) if the entrypoint is missing, the execution fails,
    /// or the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::{json_args, Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// # tokio_test::block_on(async {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "rustyscript.register_entrypoint(() => 'test')");
    /// let module = runtime.load_module(&module).await?;
    ///
    /// // Run the entrypoint and handle the result
    /// let value: String = runtime.call_entrypoint(&module, json_args!()).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub async fn call_entrypoint<T>(
        &mut self,
        module_context: &ModuleHandle,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        if let Some(entrypoint) = module_context.entrypoint() {
            let value: serde_json::Value = self
                .0
                .call_function_by_ref_async(module_context, entrypoint.clone(), args)
                .await?;
            Ok(serde_json::from_value(value)?)
        } else {
            Err(Error::MissingEntrypoint(module_context.module().clone()))
        }
    }

    /// Loads a module into a new runtime, executes the entry function and returns the
    /// result of the module's execution, deserialized into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    /// * `runtime_options` - Options for the creation of the runtime
    /// * `entrypoint_args` - Arguments to pass to the entrypoint function
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the entrypoint execution (`T`)
    /// if successful, or an error (`Error`) if the entrypoint is missing, the execution fails,
    /// or the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use rustyscript::{json_args, Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// # tokio_test::block_on(async {
    /// let module = Module::new("test.js", "rustyscript.register_entrypoint(() => 2)");
    /// let value: usize = Runtime::execute_module(&module, vec![], Default::default(), json_args!()).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub async fn execute_module<T>(
        module: &Module,
        side_modules: Vec<&Module>,
        runtime_options: RuntimeOptions,
        entrypoint_args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let mut runtime = Runtime::new(runtime_options)?;
        let module = runtime.load_modules(module, side_modules).await?;
        let value: T = runtime.call_entrypoint(&module, entrypoint_args).await?;
        Ok(value)
    }
}

#[cfg(test)]
mod test_runtime {
    use crate::json_args;
    use std::time::Duration;

    use super::*;
    use deno_core::extension;

    #[test]
    fn test_new() {
        Runtime::new(Default::default()).expect("Could not create the runtime");

        extension!(test_extension);
        Runtime::new(RuntimeOptions {
            extensions: vec![test_extension::init_ops_and_esm()],
            ..Default::default()
        })
        .expect("Could not create runtime with extensions");
    }

    #[test]
    fn test_into_arg() {
        assert_eq!(2, Runtime::into_arg(2));
        assert_eq!("test", Runtime::into_arg("test"));
        assert_ne!("test", Runtime::into_arg(2));
    }

    #[tokio::test]
    async fn test_get_value() {
        let module = Module::new(
            "test.js",
            "
            globalThis.a = 2;
            export const b = 'test';
            export const fnc = null;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&module, vec![])
            .await
            .expect("Could not load module");

        assert_eq!(
            2,
            runtime
                .get_value::<usize>(&module, "a")
                .await
                .expect("Could not find global")
        );
        assert_eq!(
            "test",
            runtime
                .get_value::<String>(&module, "b")
                .await
                .expect("Could not find export")
        );
        runtime
            .get_value::<Undefined>(&module, "c")
            .await
            .expect_err("Could not detect null");
        runtime
            .get_value::<Undefined>(&module, "d")
            .await
            .expect_err("Could not detect undeclared");
    }

    #[tokio::test]
    async fn test_load_module() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            rustyscript.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .await
            .expect("Could not load module");
        assert_ne!(0, module.id());

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module1 = Module::new(
            "importme.js",
            "
            export const value = 2;
        ",
        );
        let module2 = Module::new(
            "test.js",
            "
            import { value } from './importme.js';
            rustyscript.register_entrypoint(() => value);
        ",
        );
        runtime
            .load_module(&module1)
            .await
            .expect("Could not load modules");
        let module = runtime
            .load_module(&module2)
            .await
            .expect("Could not load modules");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .await
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            timeout: Duration::from_millis(50),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            await new Promise(r => setTimeout(r, 2000));
        ",
        );
        runtime
            .load_modules(&module, vec![])
            .await
            .expect_err("Did not interupt after timeout");
    }

    #[tokio::test]
    async fn test_load_modules() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            rustyscript.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .await
            .expect("Could not load module");
        assert_ne!(0, module.id());

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module1 = Module::new(
            "importme.js",
            "
            export const value = 2;
        ",
        );
        let module2 = Module::new(
            "test.js",
            "
            import { value } from './importme.js';
            rustyscript.register_entrypoint(() => value);
        ",
        );
        let module = runtime
            .load_modules(&module2, vec![&module1])
            .await
            .expect("Could not load modules");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .await
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            timeout: Duration::from_millis(50),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            await new Promise(r => setTimeout(r, 5000));
        ",
        );
        runtime
            .load_modules(&module, vec![])
            .await
            .expect_err("Did not interupt after timeout");
    }

    #[tokio::test]
    async fn test_call_entrypoint() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            rustyscript.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .await
            .expect("Could not load module");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .await
            .expect("Could not call registered fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            default_entrypoint: Some("load".to_string()),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            export const load = () => 2;
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .await
            .expect("Could not load module");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .await
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            export const load = () => 2;
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .await
            .expect("Could not load module");
        runtime
            .call_entrypoint::<Undefined>(&module, json_args!())
            .await
            .expect_err("Did not detect no entrypoint");
    }

    #[tokio::test]
    async fn test_execute_module() {
        let module = Module::new(
            "test.js",
            "
            rustyscript.register_entrypoint(() => 2);
        ",
        );
        let value: usize =
            Runtime::execute_module(&module, vec![], Default::default(), json_args!())
                .await
                .expect("Could not exec module");
        assert_eq!(2, value);

        let module = Module::new(
            "test.js",
            "
            function load() { return 2; }
        ",
        );
        Runtime::execute_module::<Undefined>(&module, vec![], Default::default(), json_args!())
            .await
            .expect_err("Could not detect no entrypoint");
    }

    #[tokio::test]
    async fn call_function() {
        let module = Module::new(
            "test.js",
            "
            globalThis.fna = (i) => i;
            export function fnb() { return 'test'; }
            export const fnc = 2;
            export const fne = () => {};
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&module, vec![])
            .await
            .expect("Could not load module");

        let result: usize = runtime
            .call_function(&module, "fna", json_args!(2))
            .await
            .expect("Could not call global");
        assert_eq!(2, result);

        let result: String = runtime
            .call_function(&module, "fnb", json_args!())
            .await
            .expect("Could not call export");
        assert_eq!("test", result);

        runtime
            .call_function::<Undefined>(&module, "fnc", json_args!())
            .await
            .expect_err("Did not detect non-function");
        runtime
            .call_function::<Undefined>(&module, "fnd", json_args!())
            .await
            .expect_err("Did not detect undefined");
        runtime
            .call_function::<Undefined>(&module, "fne", json_args!())
            .await
            .expect("Did not allow undefined return");
    }
}
