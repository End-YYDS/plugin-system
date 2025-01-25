use std::path::{Path, PathBuf};

use crate::types::{PluginCreate, PluginEntry, Plugins, Routes};
use actix_web::web;
use plugin_lib::Plugin;
#[derive(Default, Debug)]
pub struct PluginManager {
    plugins: Plugins,
    routes: Routes,
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(plugin_dir: P) -> Self {
        Self {
            plugins: Plugins::default(),
            routes: Routes::default(),
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
        }
    }
    pub fn load_all_plugins(&mut self) -> std::io::Result<()> {
        for entry in std::fs::read_dir(&self.plugin_dir)? {
            let path = entry?.path();
            if path
                .extension()
                .map_or(false, |ext| ext == "so" || ext == "dll" || ext == "dylib")
            {
                unsafe {
                    self.load_plugin(&path)?;
                }
            }
        }
        Ok(())
    }
    pub fn get_routes_json(&self) -> String {
        let routes = self.routes.read().unwrap();
        serde_json::to_string_pretty(&*routes).unwrap()
    }
    // pub fn load_plugin(&self, name: String, plugin: Box<dyn Plugin>) {
    //     let scope = plugin.scope();
    //     let routes: Vec<String> = plugin
    //         .register_routes()
    //         .iter()
    //         .map(|(path, _)| path.clone())
    //         .collect();

    //     self.routes.write().unwrap().insert(scope, routes);
    //     self.plugins.write().unwrap().insert(name, plugin);
    // }
    pub unsafe fn load_plugin(&mut self, path: &Path) -> std::io::Result<()> {
        let library = libloading::Library::new(path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to load library: {}", e),
            )
        })?;
        let creator: libloading::Symbol<PluginCreate> =
            library.get(b"_create_plugin").map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to load symbol: {}", e),
                )
            })?;
        let raw = creator();
        let plugin = Box::from_raw(raw);
        let name = plugin.name().to_string();
        let scope = plugin.scope();
        let routes: Vec<String> = plugin
            .register_routes()
            .iter()
            .map(|(path, _)| path.into())
            .collect();
        self.routes.write().unwrap().insert(scope, routes);
        self.plugins
            .write()
            .unwrap()
            .insert(name.clone(), PluginEntry { plugin, library });
        Ok(())
    }
    pub fn configure_routes(&self, cfg: &mut web::ServiceConfig) {
        let plugins = self.plugins.read().unwrap();
        for entry in plugins.values() {
            let scope_path = entry.plugin.scope();
            cfg.service(web::scope(&scope_path).configure(|app| {
                for (path, route_factory) in entry.plugin.register_routes() {
                    app.service(web::resource(&path).route(route_factory()));
                }
            }));
        }
    }
    #[allow(unused)]
    pub fn get_plugin_info(&self, name: &str, field: &str) -> Option<String> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(name).map(|entry| match field {
            "name" => entry.plugin.name().to_string(),
            "version" => entry.plugin.version().to_string(),
            "description" => entry.plugin.description().to_string(),
            "scope" => entry.plugin.scope(),
            _ => "Unknown".to_string(),
        })
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        let mut plugins = self.plugins.write().unwrap();
        plugins.clear();
    }
}
