///
/// This example is meant to demonstrate the use of the import utility
///
/// It acts as a wrapper around a runtime with a single loaded module
/// and is meant to simplify usecases where multiple JS sources isn't
/// needed
///
use rustyscript::{json_args, Error, JsFunction, Undefined};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut module = rustyscript::import("examples/javascript/example_module.js").await?;

    // We can list all of this module's exports
    assert_eq!(
        module.keys(),
        vec!["MY_FAVOURITE_FOOD", "addBook", "listBooks"]
    );

    // Or ensure a given export is a function
    assert!(module.is_callable("addBook").await);

    // We can grab constants
    let value: String = module.get("MY_FAVOURITE_FOOD").await?;
    assert_eq!(value, "saskatoonberries");

    // We can call functions - the Undefined here just means we don't care
    // what type it returns
    module
        .call::<Undefined>("addBook", json_args!("My Favorite Martian"))
        .await?;
    module
        .call::<Undefined>(
            "addBook",
            json_args!("The Ultimate Saskatoon Berry Cookbook"),
        )
        .await?;

    // Functions can even be stored for later!
    // They can only be used on the runtime that made them, however
    let function: JsFunction = module.get("listBooks").await?;

    // The stored function can then be called!
    // Any serializable type can be retrieved as a function result or value
    let books: Vec<String> = module.call_stored(&function, json_args!()).await?;
    assert_eq!(
        books,
        vec![
            "My Favorite Martian",
            "The Ultimate Saskatoon Berry Cookbook"
        ]
    );

    Ok(())
}
