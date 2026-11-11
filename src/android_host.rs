use gpui::{App, AppContext, WindowOptions};
use gpui_mobile::android::{host, jni as platform_jni};
use jni::{objects::JObject, sys::*};
use std::sync::atomic::{AtomicI32, Ordering};

static RENDER_THREAD_ID: AtomicI32 = AtomicI32::new(0);
enum HostEvent {
    Back,
    Theme(bool),
    KeyboardInset(f32),
}

type HostEvents = (
    smol::channel::Sender<HostEvent>,
    smol::channel::Receiver<HostEvent>,
);
static HOST_EVENTS: std::sync::OnceLock<HostEvents> = std::sync::OnceLock::new();

fn host_events() -> &'static HostEvents {
    HOST_EVENTS.get_or_init(smol::channel::unbounded)
}

pub fn observe_events(cx: &mut gpui::Context<crate::screens::Router>) {
    let receiver = host_events().1.clone();
    cx.spawn(async move |router, cx| {
        while let Ok(event) = receiver.recv().await {
            match event {
                HostEvent::Theme(dark) => {
                    let _ = router.update(cx, |router, cx| {
                        let palette = crate::screens::theme::Palette::for_dark(dark);
                        if router.palette != palette {
                            router.palette = palette;
                            cx.notify();
                        }
                    });
                }
                HostEvent::KeyboardInset(inset) => {
                    let _ = router.update(cx, |router, cx| {
                        if router.keyboard_bottom_inset != inset {
                            router.keyboard_bottom_inset = inset;
                            cx.notify();
                        }
                    });
                }
                HostEvent::Back => {
                    if router
                        .update(cx, |router, cx| {
                            if !router.go_back(cx) {
                                let result = platform_jni::with_env(|env| {
                                    let activity = platform_jni::activity(env)?;
                                    env.call_method(
                                        &activity,
                                        jni::jni_str!("gpuiBackToHome"),
                                        jni::jni_sig!("()V"),
                                        &[],
                                    )
                                    .map_err(|error| error.to_string())?;
                                    Ok(())
                                });
                                if let Err(error) = result {
                                    log::warn!("Returning to Android home failed: {error}");
                                }
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    })
    .detach();
}

#[no_mangle]
extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeBack(
    _env: *mut JNIEnv,
    _class: jclass,
) {
    let _ = host_events().0.try_send(HostEvent::Back);
}

#[no_mangle]
extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeKeyboardInset(
    _env: *mut JNIEnv,
    _class: jclass,
    inset: jfloat,
) {
    let _ = host_events().0.try_send(HostEvent::KeyboardInset(inset));
}

#[no_mangle]
extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeThemeChanged(
    _env: *mut JNIEnv,
    _class: jclass,
    dark: jboolean,
) {
    let _ = host_events().0.try_send(HostEvent::Theme(dark));
}

// gpui-mobile still links android-activity's exported glue, which requires this
// symbol even for host-driven apps. The manifest launches a plain GactActivity;
// no GPUI runtime may be created from this compatibility entry point.
#[no_mangle]
fn android_main(_app: android_activity::AndroidApp) {
    log::error!("NativeActivity entry is unsupported; use GactActivity");
}

fn launch(cx: &mut App, dark: bool) {
    let http_client = crate::api::make_client().expect("reqwest client");
    let http_client: reqwest_client::ReqwestClient = http_client.into();
    cx.set_http_client(std::sync::Arc::new(http_client));
    match cx.open_window(WindowOptions::default(), |_, cx| {
        cx.new(|cx| {
            let mut router = crate::screens::Router::new(cx);
            router.palette = crate::screens::theme::Palette::for_dark(dark);
            router
        })
    }) {
        Ok(_) => {
            let thread_id = unsafe { libc::gettid() };
            RENDER_THREAD_ID.store(thread_id, Ordering::Release);
            log::info!("host runtime initialized: render_thread={thread_id}");
        }
        Err(error) => log::error!("host open_window failed: {error:#}"),
    }
    cx.activate(true);
}

#[no_mangle]
unsafe extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeInitialize(
    raw_env: *mut JNIEnv,
    activity: jobject,
    dark: jboolean,
) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("gact"),
    );
    platform_jni::install_panic_hook();
    let mut env = unsafe { jni::EnvUnowned::from_raw(raw_env) };
    env.with_env(|env| -> jni::errors::Result<()> {
        let activity = unsafe { JObject::from_raw(env, activity) };
        platform_jni::set_host_activity(env, &activity).map_err(|error| {
            log::error!("host Activity registration failed: {error}");
            jni::errors::Error::NullPtr("host Activity registration")
        })?;
        host::start(move |cx| launch(cx, dark));
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

#[no_mangle]
unsafe extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeSurfaceCreated(
    env: *mut JNIEnv,
    _class: jclass,
    surface: jobject,
    scale: jfloat,
) {
    // The Java callback keeps Surface alive; NativeWindow acquires its own ref.
    match unsafe { ndk::native_window::NativeWindow::from_surface(env.cast(), surface) } {
        Some(window) => host::surface_created(window, scale),
        None => log::error!("host Surface has no native window"),
    }
}

#[no_mangle]
extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeSurfaceDestroyed(
    _env: *mut JNIEnv,
    _class: jclass,
) {
    host::surface_destroyed();
}

#[no_mangle]
extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeResumed(
    _env: *mut JNIEnv,
    _class: jclass,
    resumed: jboolean,
) {
    if resumed {
        host::resumed();
    } else {
        host::paused();
    }
}

#[no_mangle]
extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeRenderThreadId(
    _env: *mut JNIEnv,
    _class: jclass,
) -> jint {
    RENDER_THREAD_ID.load(Ordering::Acquire)
}

#[no_mangle]
unsafe extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeMotion(
    _env: *mut JNIEnv,
    _class: jclass,
    action: jint,
    action_index: jint,
    ids: jintArray,
    xs: jfloatArray,
    ys: jfloatArray,
) -> jboolean {
    use jni::objects::{JFloatArray, JIntArray};
    platform_jni::with_env(|env| {
        let ids = unsafe { JIntArray::from_raw(env, ids) };
        let xs = unsafe { JFloatArray::from_raw(env, xs) };
        let ys = unsafe { JFloatArray::from_raw(env, ys) };
        let count = ids.len(env).map_err(|error| error.to_string())?;
        let mut pointer_ids = vec![0; count];
        let mut x = vec![0.0; count];
        let mut y = vec![0.0; count];
        ids.get_region(env, 0, &mut pointer_ids)
            .map_err(|error| error.to_string())?;
        xs.get_region(env, 0, &mut x)
            .map_err(|error| error.to_string())?;
        ys.get_region(env, 0, &mut y)
            .map_err(|error| error.to_string())?;
        let pointers: Vec<_> = pointer_ids
            .into_iter()
            .zip(x)
            .zip(y)
            .map(|((id, x), y)| host::Pointer { id, x, y })
            .collect();
        Ok(host::motion_event(action as u32, action_index as usize, &pointers) as jboolean)
    })
    .unwrap_or(false)
}

#[no_mangle]
extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeKey(
    _env: *mut JNIEnv,
    _class: jclass,
    key: jint,
    action: jint,
    modifiers: jint,
) {
    host::key(key, action, modifiers);
}

#[no_mangle]
unsafe extern "system" fn Java_com_storytellerf_gact_GactInputActivity_nativeIme(
    _env: *mut JNIEnv,
    _class: jclass,
    session: jlong,
    kind: jint,
    text: jstring,
    start: jint,
    end: jint,
) {
    let _ = platform_jni::with_env(|env| {
        let text = unsafe { jni::objects::JString::from_raw(env, text) };
        let text = text.try_to_string(env).map_err(|error| error.to_string())?;
        host::ime_event(
            session as u64,
            kind,
            text,
            start.max(0) as usize,
            end.max(0) as usize,
        );
        Ok(())
    });
}

#[no_mangle]
unsafe extern "system" fn Java_com_storytellerf_gact_GactActivity_nativeDeepLink(
    env: *mut JNIEnv,
    class: jclass,
    url: jstring,
) {
    // Keep the upstream deeplink package's existing JNI dispatch semantics.
    unsafe {
        platform_jni::Java_dev_gpui_mobile_GpuiActivity_nativeOnDeepLink(
            env.cast(),
            class.cast(),
            url.cast(),
        );
    }
}
