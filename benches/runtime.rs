// use criterion::{async_executor::AsyncExecutor, criterion_group, criterion_main, Criterion};
// use rustyscript::{json_args, Module, Runtime, RuntimeOptions};

// async fn criterion_benchmark(c: &mut Criterion) {
//     c.bench_function("init_runtime", |b| {
//         b.iter(|| Runtime::new(Default::default()).expect("Could not create runtime"))
//     });

//     let mut runtime = Runtime::new(Default::default()).expect("Could not create runtime");
//     let mut m_id = 0;
//     c.bench_function("load_module", |b| {
//         b.iter(|| async {
//             let module = Module::new(&format!("{m_id}.js"), "export const v = 1;");
//             m_id += 1;
//             runtime
//                 .load_module(&module)
//                 .await
//                 .expect("Could not load mod");
//         })
//     });

//     // Set up a runtime for the next 2 tests
//     let mut runtime = Runtime::new(RuntimeOptions {
//         default_entrypoint: Some("test".to_string()),
//         ..Default::default()
//     })
//     .expect("Could not create runtime");
//     let modref = runtime
//         .load_module(&Module::new(
//             "test_entrypoint.js",
//             "
//         export function test() { return 1; }
//     ",
//         ))
//         .await
//         .expect("Could not load mod");

//     c.bench_function("call_entrypoint", |b| {
//         b.iter(|| {
//             let _: usize = runtime
//                 .call_entrypoint(&modref, json_args!())
//                 .expect("could not call function");
//         })
//     });

//     c.bench_function("call_function", |b| {
//         b.iter(|| {
//             let _: usize = runtime
//                 .call_function(&modref, "test", json_args!())
//                 .expect("could not call function");
//         })
//     });

//     c.bench_function("call_function_with_args", |b| {
//         b.iter(|| {
//             let _: usize = runtime
//                 .call_function(&modref, "test", json_args!("test", 1, false))
//                 .expect("could not call function");
//         })
//     });
// }

// criterion_group!(benches, criterion_benchmark);
// criterion_main!(benches);

fn main() {
    println!("Hello, world!");
}
