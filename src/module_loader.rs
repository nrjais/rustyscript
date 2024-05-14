use crate::{module_cache::ModuleCacheProvider, transpiler};
use deno_core::{
    anyhow::{self, anyhow},
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
};
use std::{collections::HashSet, rc::Rc, sync::Mutex};

pub struct RustyLoader {
    fs_whlist: Mutex<HashSet<String>>,
    cache_provider: Rc<dyn ModuleCacheProvider>,
}

#[allow(unreachable_code)]
impl ModuleLoader for RustyLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, anyhow::Error> {
        let url = deno_core::resolve_import(specifier, referrer)?;
        if referrer == "." {
            self.whitelist_add(url.as_str());
        }

        // We check permissions first
        match url.scheme() {
            // Remote fetch imports
            "https" | "http" => {
                #[cfg(not(feature = "url_import"))]
                return Err(anyhow!("web imports are not allowed here: {specifier}"));
            }

            // Dynamic FS imports
            "file" =>
            {
                #[cfg(not(feature = "fs_import"))]
                if !self.whitelist_has(url.as_str()) {
                    return Err(anyhow!("requested module is not loaded: {specifier}"));
                }
            }

            _ if specifier.starts_with("ext:") => {
                // Extension import - allow
            }

            _ => {
                return Err(anyhow!(
                    "unrecognized schema for module import: {specifier}"
                ));
            }
        }

        Ok(url)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> deno_core::ModuleLoadResponse {
        // We check permissions first
        match module_specifier.scheme() {
            // Remote fetch imports
            #[cfg(feature = "url_import")]
            "https" | "http" => {
                let future = Self::load_external(
                    module_specifier.clone(),
                    Rc::clone(&self.cache_provider),
                    |specifier| async {
                        let response = reqwest::get(specifier).await?;
                        Ok(response.text().await?)
                    },
                );
                ModuleLoadResponse::Async(Box::pin(future))
            }

            // FS imports
            "file" => {
                let future = Self::load_external(
                    module_specifier.clone(),
                    Rc::clone(&self.cache_provider),
                    |specifier| async move {
                        let path = specifier
                            .to_file_path()
                            .map_err(|_| anyhow!("`{specifier}` is not a valid file URL."))?;
                        Ok(tokio::fs::read_to_string(path).await?)
                    },
                );
                ModuleLoadResponse::Async(Box::pin(future))
            }

            _ => ModuleLoadResponse::Sync(Err(anyhow!(
                "{} imports are not allowed here: {}",
                module_specifier.scheme(),
                module_specifier.as_str()
            ))),
        }
    }
}

#[allow(dead_code)]
impl RustyLoader {
    pub fn new(cache_provider: Rc<dyn ModuleCacheProvider>) -> Self {
        Self {
            fs_whlist: Mutex::new(Default::default()),
            cache_provider,
        }
    }

    pub fn whitelist_add(&self, specifier: &str) {
        if let Ok(mut whitelist) = self.fs_whlist.lock() {
            whitelist.insert(specifier.to_string());
        }
    }

    pub fn whitelist_has(&self, specifier: &str) -> bool {
        if let Ok(whitelist) = self.fs_whlist.lock() {
            whitelist.contains(specifier)
        } else {
            false
        }
    }

    async fn load_external<F, Fut>(
        ms: ModuleSpecifier,
        cp: Rc<dyn ModuleCacheProvider>,
        handler: F,
    ) -> Result<ModuleSource, deno_core::error::AnyError>
    where
        F: Fn(ModuleSpecifier) -> Fut,
        Fut: std::future::Future<Output = Result<String, deno_core::error::AnyError>>,
    {
        match cp.get(&ms).await {
            Some(source) => Ok(source),
            _ => {
                let module_type = if ms.path().ends_with(".json") {
                    ModuleType::Json
                } else {
                    ModuleType::JavaScript
                };

                let code = handler(ms.clone()).await?;
                let code = transpiler::transpile(&ms, &code)?;

                let source = ModuleSource::new(
                    module_type,
                    ModuleSourceCode::String(code.into()),
                    &ms,
                    None,
                );

                cp.set(&ms, cp.clone_source(&ms, &source)).await;

                Ok(source)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{module_cache::MemoryModuleCacheProvider, traits::ToModuleSpecifier};

    #[tokio::test]
    async fn test_loader() {
        let cache_provider = MemoryModuleCacheProvider::default();
        let specifier = "file:///test.ts".to_module_specifier().unwrap();
        let source = ModuleSource::new(
            ModuleType::JavaScript,
            ModuleSourceCode::String("console.log('Hello, World!')".to_string().into()),
            &specifier,
            None,
        );

        cache_provider
            .set(&specifier, cache_provider.clone_source(&specifier, &source))
            .await;

        let cached_source = cache_provider
            .get(&specifier)
            .await
            .expect("Expected to get cached source");

        let loader = RustyLoader::new(Rc::new(cache_provider));
        let response = loader.load(
            &specifier,
            None,
            false,
            deno_core::RequestedModuleType::None,
        );
        match response {
            ModuleLoadResponse::Async(future) => {
                let source = future.await.expect("Expected to get source");

                let source = if let ModuleSourceCode::String(s) = source.code {
                    s
                } else {
                    panic!("Unexpected source code type");
                };
                let cached_source = if let ModuleSourceCode::String(s) = cached_source.code {
                    s
                } else {
                    panic!("Unexpected source code type");
                };
                assert_eq!(source, cached_source);
            }
            _ => panic!("Unexpected response"),
        }
    }
}
