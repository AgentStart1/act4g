package com.storytellerf.gact;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.os.Bundle;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.SystemClock;
import android.view.SurfaceView;

import androidx.core.view.ViewCompat;
import androidx.core.view.WindowInsetsCompat;

import java.lang.reflect.Method;

/** Exercises the native Activity lifecycle without replacing the app's stored data. */
public final class ActivityLifecycleTest extends Instrumentation {
    private static final String ACTIVITY = "com.storytellerf.gact.GactActivity";
    private static final long TIMEOUT_MS = 30000;
    private String scenario;
    private long backgroundMs;

    @Override
    public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        scenario = arguments.getString("scenario", "recreate");
        backgroundMs = Long.parseLong(arguments.getString("backgroundMs", "30000"));
        start();
    }

    @Override
    public void onStart() {
        Bundle result = new Bundle();
        try {
            Intent launch = new Intent(Intent.ACTION_MAIN)
                    .setClassName(getTargetContext().getPackageName(), ACTIVITY)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            Activity activity = startActivitySync(launch);
            awaitReady(activity);
            if ("recreate".equals(scenario)) {
                recreate(activity);
            } else if ("background".equals(scenario)) {
                backgroundResume(activity, launch);
            } else if ("theme".equals(scenario)) {
                themeFollowsSystem(activity);
            } else if ("keyboard".equals(scenario)) {
                keyboardKeepsSurfaceGeometry(activity);

            } else {
                throw new IllegalArgumentException("Unknown scenario: " + scenario);
            }
            result.putString("stream", "PASS: " + scenario + "\n");
            finish(Activity.RESULT_OK, result);
        } catch (Throwable failure) {
            result.putString("stream", "FAIL: " + android.util.Log.getStackTraceString(failure));
            finish(Activity.RESULT_CANCELED, result);
        }
    }

    private String shell(String command) throws Exception {
        try (android.os.ParcelFileDescriptor fd = getUiAutomation().executeShellCommand(command);
                java.io.FileInputStream stream = new java.io.FileInputStream(fd.getFileDescriptor())) {
            java.io.ByteArrayOutputStream output = new java.io.ByteArrayOutputStream();
            byte[] buffer = new byte[1024];
            int count;
            while ((count = stream.read(buffer)) != -1) output.write(buffer, 0, count);
            return output.toString("UTF-8").trim();
        }
    }

    private void themeFollowsSystem(Activity activity) throws Exception {
        String original = shell("cmd uimode night").replace("Night mode: ", "").trim();
        if (!java.util.Arrays.asList("yes", "no", "auto", "custom").contains(original)) {
            throw new AssertionError("Unknown original night mode: " + original);
        }
        int thread = renderThreadId(activity);
        try {
            for (boolean dark : new boolean[] {false, true, false}) {
                shell("cmd uimode night " + (dark ? "yes" : "no"));
                int expected = dark ? 0xFF0D1117 : 0xFFFFFFFF;
                long deadline = SystemClock.uptimeMillis() + TIMEOUT_MS;
                boolean matched = false;
                do {
                    boolean[] resourcesMatch = {false};
                    int[] sample = new int[2];
                    runOnMainSync(() -> {
                        android.view.View content = activity.findViewById(android.R.id.content);
                        int[] location = new int[2];
                        content.getLocationOnScreen(location);
                        sample[0] = location[0] + 4;
                        sample[1] = location[1] + content.getHeight() / 2;
                        androidx.core.view.WindowInsetsControllerCompat bars =
                                androidx.core.view.WindowCompat.getInsetsController(
                                        activity.getWindow(), activity.getWindow().getDecorView());
                        resourcesMatch[0] = activity.getColor(R.color.gpui_surface) == expected
                                && bars.isAppearanceLightStatusBars() == !dark
                                && bars.isAppearanceLightNavigationBars() == !dark;
                    });
                    Bitmap frame = getUiAutomation().takeScreenshot();
                    if (frame != null) {
                        matched = resourcesMatch[0] && frame.getPixel(sample[0], sample[1]) == expected;
                        frame.recycle();
                    }
                    if (matched) break;
                    SystemClock.sleep(100);
                } while (SystemClock.uptimeMillis() < deadline);
                if (!matched) throw new AssertionError("Native frame or system bars did not follow dark=" + dark);
                if (thread != renderThreadId(activity)) throw new AssertionError("Theme change restarted native runtime");
            }
        } finally {
            shell("cmd uimode night " + original);
            runOnMainSync(activity::finish);
        }
    }

    private void keyboardKeepsSurfaceGeometry(Activity activity) throws Exception {
        GactActivity host = (GactActivity) activity;
        SurfaceView surface = (SurfaceView) ((android.view.ViewGroup)
                activity.findViewById(android.R.id.content)).getChildAt(0);
        Rect original = new Rect(surface.getHolder().getSurfaceFrame());
        Bitmap frame = getUiAutomation().takeScreenshot();
        try {
            runOnMainSync(() -> host.gpuiShowKeyboard(0, 1));
            awaitKeyboard(surface, true);
            runOnMainSync(() -> {
                if (!original.equals(surface.getHolder().getSurfaceFrame())
                        || surface.getWidth() != original.width()
                        || surface.getHeight() != original.height()) {
                    throw new AssertionError("IME resized the GPU surface");
                }
            });
        } finally {
            runOnMainSync(() -> host.gpuiHideKeyboard(1));
        }
        awaitKeyboard(surface, false);
        awaitRenderedLike(frame);
        frame.recycle();
        runOnMainSync(activity::finish);
    }

    private void awaitKeyboard(SurfaceView surface, boolean visible) {
        long deadline = SystemClock.uptimeMillis() + TIMEOUT_MS;
        boolean[] matched = new boolean[1];
        do {
            runOnMainSync(() -> {
                WindowInsetsCompat insets = ViewCompat.getRootWindowInsets(surface);
                matched[0] = insets != null
                        && insets.isVisible(WindowInsetsCompat.Type.ime()) == visible;
            });
            if (matched[0]) return;
            SystemClock.sleep(50);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("IME visibility did not become " + visible);
    }

    private void recreate(Activity activity) throws Exception {
        int pid = android.os.Process.myPid();
        int renderThread = renderThreadId(activity);
        Bitmap frame = getUiAutomation().takeScreenshot();
        // Each new Activity must have a usable native window in the same process.
        for (int generation = 1; generation <= 10; generation++) {
            ActivityMonitor monitor = addMonitor(ACTIVITY, null, false);
            Activity previous = activity;
            try {
                runOnMainSync(previous::recreate);
                activity = monitor.waitForActivityWithTimeout(TIMEOUT_MS);
                if (activity == null || activity == previous) {
                    throw new AssertionError("Activity was not recreated");
                }
                awaitReady(activity);
                if (renderThreadId(activity) != renderThread) {
                    throw new AssertionError("GPUI render thread changed during recreation");
                }
                assertRenderedLike(frame);
                if (android.os.Process.myPid() != pid) {
                    throw new AssertionError("Process restarted during recreation");
                }
            } finally {
                removeMonitor(monitor);
            }
        }
        Activity last = activity;
        runOnMainSync(last::finish);
    }

    private void backgroundResume(Activity activity, Intent launch) throws Exception {
        int renderThread = renderThreadId(activity);
        Bitmap frame = getUiAutomation().takeScreenshot();
        Intent home = new Intent(Intent.ACTION_MAIN)
                .addCategory(Intent.CATEGORY_HOME)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        getTargetContext().startActivity(home);
        long deadline = SystemClock.uptimeMillis() + TIMEOUT_MS;
        boolean[] focused = new boolean[1];
        do {
            runOnMainSync(() -> focused[0] = activity.hasWindowFocus());
            if (!focused[0]) {
                break;
            }
            SystemClock.sleep(50);
        } while (SystemClock.uptimeMillis() < deadline);
        if (focused[0]) {
            throw new AssertionError("Activity did not enter background");
        }
        SystemClock.sleep(backgroundMs);
        getTargetContext().startActivity(launch);
        awaitReady(activity);
        if (renderThreadId(activity) != renderThread) {
            throw new AssertionError("GPUI render thread changed during background resume");
        }
        assertRenderedLike(frame);
        runOnMainSync(activity::finish);
    }

    private int renderThreadId(Activity activity) throws Exception {
        Method thread = activity.getClass().getDeclaredMethod("nativeRenderThreadId");
        thread.setAccessible(true);
        int id = (Integer) thread.invoke(null);
        if (id == 0) throw new AssertionError("GPUI has no render thread");
        return id;
    }

    private void awaitRenderedLike(Bitmap before) {
        long deadline = SystemClock.uptimeMillis() + TIMEOUT_MS;
        int matchingFrames = 0;
        String difference;
        do {
            difference = renderedDifference(before);
            matchingFrames = difference == null ? matchingFrames + 1 : 0;
            if (matchingFrames >= 3) return;
            SystemClock.sleep(100);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("IME layout failed to settle: " + difference);
    }

    private void assertRenderedLike(Bitmap before) {
        String difference = renderedDifference(before);
        if (difference != null) throw new AssertionError(difference);
    }

    private String renderedDifference(Bitmap before) {
        Bitmap after = getUiAutomation().takeScreenshot();
        if (before == null || after == null || before.getWidth() != after.getWidth()
                || before.getHeight() != after.getHeight()) {
            if (after != null) after.recycle();
            return "Cannot compare rendered windows";
        }
        // Ignore system bars and compare distinctive content, so a focused black
        // Surface cannot pass solely because its background matches the theme.
        int background = before.getPixel(8, before.getHeight() / 2);
        int content = 0;
        int matches = 0;
        for (int y = before.getHeight() / 5; y < before.getHeight() * 4 / 5; y += 4) {
            for (int x = 8; x < before.getWidth() - 8; x += 4) {
                int expected = before.getPixel(x, y);
                if (colorDistance(expected, background) > 100) {
                    content++;
                    if (colorDistance(expected, after.getPixel(x, y)) < 60) matches++;
                }
            }
        }
        after.recycle();
        if (content < 50 || matches < content * 0.9) {
            return "Rendered content did not survive lifecycle transition: " + matches + "/" + content;
        }
        return null;
    }

    private int colorDistance(int first, int second) {
        return Math.abs(((first >> 16) & 255) - ((second >> 16) & 255))
                + Math.abs(((first >> 8) & 255) - ((second >> 8) & 255))
                + Math.abs((first & 255) - (second & 255));
    }

    private void awaitReady(Activity activity) throws Exception {
        Method ready = activity.getClass().getDeclaredMethod("isNativeReady");
        ready.setAccessible(true);
        long deadline = SystemClock.uptimeMillis() + TIMEOUT_MS;
        boolean[] usable = new boolean[1];
        do {
            runOnMainSync(() -> {
                try {
                    usable[0] = !activity.isDestroyed() && activity.hasWindowFocus()
                            && (Boolean) ready.invoke(activity);
                } catch (ReflectiveOperationException error) {
                    throw new AssertionError(error);
                }
            });
            if (usable[0]) {
                // Give the native loop time to surface a crash after initialization.
                SystemClock.sleep(1000);
                return;
            }
            SystemClock.sleep(50);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native window did not become ready");
    }
}
