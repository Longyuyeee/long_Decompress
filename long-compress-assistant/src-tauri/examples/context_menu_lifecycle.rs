#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    use long_compress_assistant::system_integration::context_menu::{
        register_context_menu, unregister_context_menu,
    };

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("register") => {
            let app_path = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("register requires an application path"))?;
            register_context_menu(&app_path)?;
            println!("registered context menus for {app_path}");
        }
        Some("unregister") => {
            unregister_context_menu()?;
            println!("unregistered context menus");
        }
        _ => anyhow::bail!(
            "usage: cargo run --example context_menu_lifecycle -- <register APP_PATH|unregister>"
        ),
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("context menu lifecycle checks are only available on Windows");
}
