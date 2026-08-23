#[cfg(target_os = "android")]
const TOKEN_KEY: &str = "github_oauth_token";

#[cfg(target_os = "android")]
pub fn load_token() -> Option<String> {
    use jni::objects::{JObject, JString, JValue};

    gpui_mobile::android::jni::with_env(|env| {
        let key = env
            .new_string(TOKEN_KEY)
            .map_err(|error| error.to_string())?;
        let value: JObject = env
            .call_static_method(
                jni::jni_str!("dev/gpui/mobile/GpuiSecureStorage"),
                jni::jni_str!("getString"),
                jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(&key)],
            )
            .and_then(|value| value.l())
            .map_err(|error| error.to_string())?;

        if value.is_null() {
            Ok(None)
        } else {
            let value = JString::cast_local(env, value).map_err(|error| error.to_string())?;
            value
                .try_to_string(env)
                .map(Some)
                .map_err(|error| error.to_string())
        }
    })
    .unwrap_or_else(|error| {
        log::warn!("Failed to load the saved GitHub session: {error}");
        None
    })
}

#[cfg(not(target_os = "android"))]
pub fn load_token() -> Option<String> {
    None
}

#[cfg(target_os = "android")]
pub fn save_token(token: &str) -> Result<(), String> {
    use jni::objects::JValue;

    gpui_mobile::android::jni::with_env(|env| {
        let key = env
            .new_string(TOKEN_KEY)
            .map_err(|error| error.to_string())?;
        let token = env.new_string(token).map_err(|error| error.to_string())?;
        let saved = env
            .call_static_method(
                jni::jni_str!("dev/gpui/mobile/GpuiSecureStorage"),
                jni::jni_str!("putString"),
                jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Z"),
                &[JValue::Object(&key), JValue::Object(&token)],
            )
            .and_then(|value| value.z())
            .map_err(|error| error.to_string())?;

        if saved {
            Ok(())
        } else {
            Err("Android Keystore rejected the credential".to_string())
        }
    })
}

#[cfg(not(target_os = "android"))]
pub fn save_token(_token: &str) -> Result<(), String> {
    Err("Secure credential storage is unavailable on this platform".to_string())
}

#[cfg(target_os = "android")]
pub fn clear_token() -> Result<(), String> {
    use jni::objects::JValue;

    gpui_mobile::android::jni::with_env(|env| {
        let key = env
            .new_string(TOKEN_KEY)
            .map_err(|error| error.to_string())?;
        let removed = env
            .call_static_method(
                jni::jni_str!("dev/gpui/mobile/GpuiSecureStorage"),
                jni::jni_str!("remove"),
                jni::jni_sig!("(Ljava/lang/String;)Z"),
                &[JValue::Object(&key)],
            )
            .and_then(|value| value.z())
            .map_err(|error| error.to_string())?;

        if removed {
            Ok(())
        } else {
            Err("Failed to remove the saved credential".to_string())
        }
    })
}

#[cfg(not(target_os = "android"))]
pub fn clear_token() -> Result<(), String> {
    Ok(())
}
