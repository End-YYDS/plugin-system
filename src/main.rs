mod manager;
mod types;
use actix_web::{web, App, HttpServer, Responder};
use console::Style;
use manager::PluginManager;

async fn get_routes(plugin_manager: web::Data<PluginManager>) -> impl Responder {
    plugin_manager.get_routes_json()
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let plugin_dir = std::env::current_dir()?.join("plugins");
    if !plugin_dir.exists() {
        std::fs::create_dir_all(&plugin_dir)?;
    }
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let blue = Style::new().blue();
    let prefix = "127.0.0.1";
    let port = 8080;
    let target = format!("{}:{}", prefix, port);
    let mut manager = PluginManager::new(plugin_dir);
    manager.load_all_plugins()?;
    let plugin_manager = web::Data::new(manager);
    println!(
        "\nServer ready at {}",
        blue.apply_to(format!("http://{}", &target))
    );
    HttpServer::new(move || {
        App::new()
            .app_data(plugin_manager.clone())
            .configure(|cfg| plugin_manager.configure_routes(cfg))
            .route("/routes", web::get().to(get_routes))
    })
    .bind(&target)?
    .run()
    .await
}
