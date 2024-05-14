use std::{collections::HashMap, path::PathBuf};

use deno_core::{
    parking_lot::Mutex, ModuleCodeBytes, ModuleSource, ModuleSourceCode, ModuleSpecifier,
};
use tokio::fs;

/// Module cache provider trait
/// Implement this trait to provide a custom module cache
/// You will need to use interior due to the deno's loader trait
/// Default cache for the loader is in-memory
#[async_trait::async_trait]
pub trait ModuleCacheProvider {
    /// Set a module source in the cache
    async fn set(&self, specifier: &ModuleSpecifier, source: ModuleSource);
    /// Get a module source from the cache
    async fn get(&self, specifier: &ModuleSpecifier) -> Option<ModuleSource>;

    /// Clone a module source
    fn clone_source(&self, specifier: &ModuleSpecifier, source: &ModuleSource) -> ModuleSource {
        ModuleSource::new(
            source.module_type.clone(),
            match &source.code {
                ModuleSourceCode::String(s) => ModuleSourceCode::String(s.to_string().into()),
                ModuleSourceCode::Bytes(b) => {
                    ModuleSourceCode::Bytes(ModuleCodeBytes::Boxed(b.to_vec().into()))
                }
            },
            specifier,
            source.code_cache.clone(),
        )
    }
}

#[async_trait::async_trait]
impl ModuleCacheProvider for () {
    async fn set(&self, _: &ModuleSpecifier, _: ModuleSource) {}

    async fn get(&self, _: &ModuleSpecifier) -> Option<ModuleSource> {
        None
    }
}

/// Default in-memory module cache provider
#[derive(Default)]
pub struct MemoryModuleCacheProvider(Mutex<HashMap<ModuleSpecifier, ModuleSource>>);

#[async_trait::async_trait]
impl ModuleCacheProvider for MemoryModuleCacheProvider {
    async fn set(&self, specifier: &ModuleSpecifier, source: ModuleSource) {
        let cache = &mut self.0.lock();
        cache.insert(specifier.clone(), source);
    }

    async fn get(&self, specifier: &ModuleSpecifier) -> Option<ModuleSource> {
        let cache = &self.0.lock();
        let source = cache.get(specifier)?;
        Some(Self::clone_source(self, specifier, source))
    }
}

/// Default in-memory module cache provider
pub struct FSModuleCacheProvider {
    root: PathBuf,
    cache: MemoryModuleCacheProvider,
}

impl FSModuleCacheProvider {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            cache: MemoryModuleCacheProvider::default(),
        }
    }
}

#[async_trait::async_trait]
impl ModuleCacheProvider for FSModuleCacheProvider {
    async fn set(&self, specifier: &ModuleSpecifier, source: ModuleSource) {
        let source_str = match &source.code {
            ModuleSourceCode::String(s) => s.to_string(),
            ModuleSourceCode::Bytes(b) => {
                let b = b.to_vec();
                String::from_utf8(b).unwrap()
            }
        };

        let path = self.root.join(specifier.as_str());
        fs::write(path, source_str).await;
        self.cache.set(specifier, source).await;
    }

    async fn get(&self, specifier: &ModuleSpecifier) -> Option<ModuleSource> {
        let res = self.cache.get(specifier).await;
        if res.is_none() {
            let path = self.root.join(specifier.as_str());
            let source_str = fs::read_to_string(path).await.ok()?;
            let source =
                ModuleSource::new(ModuleSourceCode::String(source_str.into()), specifier, None);
            self.cache.set(specifier, source.clone()).await;
            Some(source)
        } else {
            res
        }
    }
}
