package com.storytellerf.gact;

import android.content.Context;
import android.content.Intent;
import android.content.res.Configuration;
import androidx.core.view.WindowCompat;
import android.media.AudioManager;
import android.os.Bundle;
import android.os.Build;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.window.OnBackInvokedCallback;
import android.window.OnBackInvokedDispatcher;

import androidx.core.splashscreen.SplashScreen;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowInsetsCompat;

/** Owns Android surfaces while android::host keeps GPUI on one render thread. */
public final class GactActivity extends GactInputActivity implements SurfaceHolder.Callback {
    private static volatile Context appContext;
    private SurfaceView surfaceView;
    private OnBackInvokedCallback backCallback;

    public static Context getAppContext() {
        return appContext;
    }

    @Override
    protected void onCreate(Bundle state) {
        SplashScreen splash = SplashScreen.installSplashScreen(this);
        super.onCreate(state);
        appContext = getApplicationContext();
        System.loadLibrary("gact");
        nativeInitialize(isDarkTheme());
        syncTheme();
        if (Build.VERSION.SDK_INT >= 33) {
            backCallback = () -> nativeBack();
            getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                    OnBackInvokedDispatcher.PRIORITY_DEFAULT, backCallback);
        }
        splash.setKeepOnScreenCondition(() -> !isNativeReady());
        setVolumeControlStream(AudioManager.STREAM_MUSIC);
        surfaceView = new SurfaceView(this) {
            @Override
            public boolean onTouchEvent(MotionEvent event) {
                int count = event.getPointerCount();
                int[] ids = new int[count];
                float[] x = new float[count];
                float[] y = new float[count];
                for (int index = 0; index < count; index++) {
                    ids[index] = event.getPointerId(index);
                    x[index] = event.getX(index);
                    y[index] = event.getY(index);
                }
                return nativeMotion(event.getActionMasked(), event.getActionIndex(), ids, x, y);
            }
        };
        surfaceView.setFocusableInTouchMode(true);
        surfaceView.getHolder().addCallback(this);
        setContentView(surfaceView);
        // Keep the GPU surface stable during IME animation. Reflow GPUI content
        // within the unobscured region instead of resizing its queued buffers.
        android.view.View decor = getWindow().getDecorView();
        ViewCompat.setOnApplyWindowInsetsListener(decor, (view, insets) -> {
            int[] location = new int[2];
            surfaceView.getLocationInWindow(location);
            int keyboardTop = view.getHeight()
                    - insets.getInsets(WindowInsetsCompat.Type.ime()).bottom;
            int overlap = Math.min(surfaceView.getHeight(),
                    Math.max(0, location[1] + surfaceView.getHeight() - keyboardTop));
            nativeKeyboardInset(overlap / getResources().getDisplayMetrics().density);
            return ViewCompat.onApplyWindowInsets(view, insets);
        });
        surfaceView.addOnLayoutChangeListener((view, left, top, right, bottom,
                oldLeft, oldTop, oldRight, oldBottom) -> ViewCompat.requestApplyInsets(decor));
        ViewCompat.requestApplyInsets(decor);
        surfaceView.requestFocus();
        deliverDeepLink(getIntent());
    }

    private boolean isDarkTheme() {
        return (getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                == Configuration.UI_MODE_NIGHT_YES;
    }

    @SuppressWarnings("deprecation")
    private void syncTheme() {
        boolean dark = isDarkTheme();
        int background = getColor(com.storytellerf.gact.R.color.gpui_surface);
        getWindow().addFlags(android.view.WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS);
        getWindow().setStatusBarColor(background);
        getWindow().setNavigationBarColor(background);
        androidx.core.view.WindowInsetsControllerCompat controller =
                WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        controller.setAppearanceLightStatusBars(!dark);
        controller.setAppearanceLightNavigationBars(!dark);
        nativeThemeChanged(dark);
    }

    @Override
    public void onConfigurationChanged(Configuration configuration) {
        super.onConfigurationChanged(configuration);
        syncTheme();
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
        attachSurface(holder);
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        attachSurface(holder);
    }

    private void attachSurface(SurfaceHolder holder) {
        nativeSurfaceCreated(holder.getSurface(), getResources().getDisplayMetrics().density);
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        // Wait for the render thread before Android releases the Surface.
        nativeSurfaceDestroyed();
    }

    @Override
    protected void onResume() {
        super.onResume();
        syncTheme();
        nativeResumed(true);
    }

    @Override
    protected void onPause() {
        nativeResumed(false);
        super.onPause();
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        int key = event.getKeyCode();
        if (hasInputFocus() || key == KeyEvent.KEYCODE_BACK
                || key == KeyEvent.KEYCODE_VOLUME_UP || key == KeyEvent.KEYCODE_VOLUME_DOWN
                || key == KeyEvent.KEYCODE_VOLUME_MUTE || KeyEvent.isMediaSessionKey(key)) {
            return super.dispatchKeyEvent(event);
        }
        if (event.getAction() == KeyEvent.ACTION_DOWN || event.getAction() == KeyEvent.ACTION_UP) {
            nativeKey(key, event.getAction(), event.getMetaState());
            return true;
        }
        return super.dispatchKeyEvent(event);
    }

    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        nativeBack();
    }

    public void gpuiBackToHome() {
        runOnUiThread(() -> moveTaskToBack(true));
    }

    @Override
    protected void onDestroy() {
        if (Build.VERSION.SDK_INT >= 33 && backCallback != null) {
            getOnBackInvokedDispatcher().unregisterOnBackInvokedCallback(backCallback);
        }
        super.onDestroy();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        deliverDeepLink(intent);
    }

    private void deliverDeepLink(Intent intent) {
        if (intent != null && intent.getData() != null) {
            nativeDeepLink(intent.getData().toString());
        }
    }

    private boolean isNativeReady() {
        return surfaceView != null && surfaceView.getHolder().getSurface().isValid()
                && nativeRenderThreadId() != 0;
    }

    private native void nativeInitialize(boolean dark);
    private static native void nativeThemeChanged(boolean dark);
    private static native void nativeBack();
    private static native void nativeSurfaceCreated(Surface surface, float scale);
    private static native void nativeSurfaceDestroyed();
    private static native void nativeKeyboardInset(float inset);
    private static native void nativeResumed(boolean resumed);
    private static native boolean nativeMotion(int action, int actionIndex, int[] ids, float[] x, float[] y);
    private static native void nativeKey(int key, int action, int modifiers);
    private static native void nativeDeepLink(String url);
    private static native int nativeRenderThreadId();
}
