package dev.gpui.mobile;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import android.util.Log;

import java.nio.charset.StandardCharsets;
import java.security.KeyStore;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** App-private credential storage backed by an AES key in Android Keystore. */
public final class GpuiSecureStorage {
    private static final String TAG = "GpuiSecureStorage";
    private static final String PREFERENCES = "act4g_secure_credentials";
    private static final String KEY_ALIAS = "act4g_credentials_key_v1";
    private static final String CIPHER = "AES/GCM/NoPadding";

    private GpuiSecureStorage() {}

    public static String getString(String key) {
        Context context = GpuiActivity.getAppContext();
        if (context == null) {
            return null;
        }

        String encoded = preferences(context).getString(key, null);
        if (encoded == null) {
            return null;
        }

        try {
            String[] parts = encoded.split(":", 2);
            if (parts.length != 2) {
                remove(key);
                return null;
            }

            byte[] iv = Base64.decode(parts[0], Base64.NO_WRAP);
            byte[] encrypted = Base64.decode(parts[1], Base64.NO_WRAP);
            Cipher cipher = Cipher.getInstance(CIPHER);
            cipher.init(Cipher.DECRYPT_MODE, getOrCreateKey(), new GCMParameterSpec(128, iv));
            return new String(cipher.doFinal(encrypted), StandardCharsets.UTF_8);
        } catch (Exception error) {
            Log.w(TAG, "Could not decrypt saved credential", error);
            remove(key);
            return null;
        }
    }

    public static boolean putString(String key, String value) {
        Context context = GpuiActivity.getAppContext();
        if (context == null) {
            return false;
        }

        try {
            Cipher cipher = Cipher.getInstance(CIPHER);
            cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey());
            byte[] encrypted = cipher.doFinal(value.getBytes(StandardCharsets.UTF_8));
            String encoded = Base64.encodeToString(cipher.getIV(), Base64.NO_WRAP)
                    + ":"
                    + Base64.encodeToString(encrypted, Base64.NO_WRAP);
            return preferences(context).edit().putString(key, encoded).commit();
        } catch (Exception error) {
            Log.e(TAG, "Could not encrypt credential", error);
            return false;
        }
    }

    public static boolean remove(String key) {
        Context context = GpuiActivity.getAppContext();
        return context != null && preferences(context).edit().remove(key).commit();
    }

    private static SharedPreferences preferences(Context context) {
        return context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE);
    }

    private static SecretKey getOrCreateKey() throws Exception {
        KeyStore keyStore = KeyStore.getInstance("AndroidKeyStore");
        keyStore.load(null);
        KeyStore.Entry existing = keyStore.getEntry(KEY_ALIAS, null);
        if (existing instanceof KeyStore.SecretKeyEntry) {
            return ((KeyStore.SecretKeyEntry) existing).getSecretKey();
        }

        KeyGenerator generator = KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_AES,
                "AndroidKeyStore"
        );
        generator.init(new KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT
        )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .build());
        return generator.generateKey();
    }
}
