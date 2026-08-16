extern crate gpui_mobile;

pub mod api;
pub mod screens;

#[cfg(target_os = "android")]
use gpui::{App, Application, WindowOptions};
#[cfg(target_os = "android")]
use gpui_mobile::android::jni;
#[cfg(target_os = "android")]
use screens::Router;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("act4g"),
    );

    jni::install_panic_hook();
    log::info!("android_main: started");

    let _platform = jni::init_platform(&app);

    let shared = match jni::shared_platform() {
        Some(s) => s,
        None => {
            log::error!("android_main: shared_platform() returned None");
            return;
        }
    };

    Application::with_platform(shared.into_rc()).run(|cx: &mut App| {
        // HTTP client used by gpui::img() to load remote avatars.
        let http_client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client");
        let http_client: reqwest_client::ReqwestClient = http_client.into();
        cx.set_http_client(std::sync::Arc::new(http_client));

        match cx.open_window(
            WindowOptions { window_bounds: None, ..Default::default() },
            |_, cx| cx.new(|_| Router::new()),
        ) {
            Ok(_) => log::info!("window opened"),
            Err(e) => log::error!("open_window failed: {e:#}"),
        }

        cx.activate(true);
    });

    log::info!("android_main: exiting");
}
