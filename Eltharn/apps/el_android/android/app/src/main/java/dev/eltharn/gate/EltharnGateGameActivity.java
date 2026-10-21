package dev.eltharn.gate;

import android.content.Intent;
import android.app.ActivityManager;
import android.app.Application;
import android.content.pm.ActivityInfo;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Process;
import android.view.InputDevice;
import android.view.MotionEvent;
import com.google.androidgamesdk.GameActivity;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;
import java.util.List;


public final class EltharnGateGameActivity extends GameActivity {
    private boolean gameStarted;
    private boolean dedicatedGameProcess;
    private boolean haveMousePosition;
    private float lastMouseX;
    private float lastMouseY;

    static {
        System.loadLibrary("eltharn_android");
    }

    @Override
    protected void onCreate(Bundle state) {
        dedicatedGameProcess = isDedicatedGameProcess();
        super.onCreate(state);
        if (!dedicatedGameProcess) {
            onNativeGameFinished("Game activity requires its dedicated process");
            return;
        }
        Intent launch = getIntent();
        String protocol = launch.getStringExtra("protocol");
        String username = launch.getStringExtra("username");
        String controls = launch.getStringExtra("controls");
        String orientation = launch.getStringExtra("orientation");
        if (protocol == null || username == null || controls == null || orientation == null) {
            onNativeGameFinished("Game launch request is incomplete");
            return;
        }
        requestGameStart(protocol, username, controls, orientation);
    }

    private native void nativeStartGame(
            String protocol, String username, String controls, String orientation);
    private native void nativeMouseMotion(double dx, double dy);
    private native void nativeMouseWheel(double x, double y);
    private native void nativeMouseButton(int button, boolean pressed);
    private native void nativeActivityDestroyed();

    @Override
    protected void onDestroy() {
        nativeActivityDestroyed();
        super.onDestroy();
        if (dedicatedGameProcess) {
            new Handler(Looper.getMainLooper()).post(() -> Process.killProcess(Process.myPid()));
        }
    }

    private boolean isDedicatedGameProcess() {
        String expected = getPackageName() + ":game";
        if (Build.VERSION.SDK_INT >= 28) {
            return expected.equals(Application.getProcessName());
        }
        ActivityManager manager = (ActivityManager) getSystemService(ACTIVITY_SERVICE);
        if (manager == null) {
            return false;
        }
        List<ActivityManager.RunningAppProcessInfo> processes = manager.getRunningAppProcesses();
        if (processes == null) {
            return false;
        }
        for (ActivityManager.RunningAppProcessInfo process : processes) {
            if (process.pid == Process.myPid()) {
                return expected.equals(process.processName);
            }
        }
        return false;
    }

    private void onNativeGameFinished(String error) {
        runOnUiThread(() -> {
            Intent result = new Intent();
            result.putExtra("error", error);
            setResult(error.isEmpty() ? RESULT_OK : RESULT_CANCELED, result);
            finish();
        });
    }

    private void enableImmersiveMode() {
        WindowInsetsControllerCompat controller = WindowCompat.getInsetsController(
                getWindow(), getWindow().getDecorView());
        controller.hide(WindowInsetsCompat.Type.systemBars());
        controller.setSystemBarsBehavior(
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus && gameStarted) {
            enableImmersiveMode();
        }
    }

    void requestGameStart(String protocol, String username, String controls, String orientation) {
        if ("landscape".equals(orientation)) {


            setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE);
        } else {
            setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_SENSOR);
        }
        gameStarted = true;
        enableImmersiveMode();
        nativeStartGame(protocol, username, controls, orientation);
    }


    @Override
    public boolean dispatchGenericMotionEvent(MotionEvent event) {
        if (gameStarted
                && (event.getSource() & InputDevice.SOURCE_MOUSE) == InputDevice.SOURCE_MOUSE) {
            int action = event.getActionMasked();
            if (action == MotionEvent.ACTION_HOVER_MOVE || action == MotionEvent.ACTION_MOVE) {
                float dx = event.getAxisValue(MotionEvent.AXIS_RELATIVE_X);
                float dy = event.getAxisValue(MotionEvent.AXIS_RELATIVE_Y);


                if (dx == 0.0f && dy == 0.0f && haveMousePosition) {
                    dx = event.getX() - lastMouseX;
                    dy = event.getY() - lastMouseY;
                }
                lastMouseX = event.getX();
                lastMouseY = event.getY();
                haveMousePosition = true;
                if (dx != 0.0f || dy != 0.0f) {
                    nativeMouseMotion(dx, dy);
                }
                return true;
            }
            if (action == MotionEvent.ACTION_SCROLL) {
                float x = event.getAxisValue(MotionEvent.AXIS_HSCROLL);
                float y = event.getAxisValue(MotionEvent.AXIS_VSCROLL);
                if (x != 0.0f || y != 0.0f) {
                    nativeMouseWheel(x, y);
                }
                return true;
            }
            if (action == MotionEvent.ACTION_BUTTON_PRESS
                    || action == MotionEvent.ACTION_BUTTON_RELEASE) {
                int button = switch (event.getActionButton()) {
                    case MotionEvent.BUTTON_PRIMARY -> 0;
                    case MotionEvent.BUTTON_SECONDARY -> 1;
                    case MotionEvent.BUTTON_TERTIARY -> 2;
                    default -> -1;
                };
                if (button >= 0) {
                    nativeMouseButton(button, action == MotionEvent.ACTION_BUTTON_PRESS);
                    return true;
                }
            }
        }
        return super.dispatchGenericMotionEvent(event);
    }
    private native void nativeTextInput(String text);

    private native void nativeBackspace();

    private native void nativeChatEnter();

    private native void nativeChatClose();


    @Override
    public boolean dispatchKeyEvent(android.view.KeyEvent event) {
        if (gameStarted && event.getAction() == android.view.KeyEvent.ACTION_DOWN) {
            int keyCode = event.getKeyCode();
            if (keyCode == android.view.KeyEvent.KEYCODE_DEL) {
                nativeBackspace();
                return true;
            }
            if (keyCode == android.view.KeyEvent.KEYCODE_ENTER
                    || keyCode == android.view.KeyEvent.KEYCODE_NUMPAD_ENTER) {


                if ((event.getFlags() & android.view.KeyEvent.FLAG_SOFT_KEYBOARD) != 0) {
                    nativeChatEnter();
                    return true;
                }
                return super.dispatchKeyEvent(event);
            }
            if (keyCode == android.view.KeyEvent.KEYCODE_BACK) {
                nativeChatClose();
                return true;
            }
            int unicodeChar = event.getUnicodeChar();
            if (unicodeChar != 0 && !Character.isISOControl(unicodeChar)
                    && Character.isValidCodePoint(unicodeChar)) {


                nativeTextInput(new String(Character.toChars(unicodeChar)));
                return true;
            }
        }
        return super.dispatchKeyEvent(event);
    }


    @Override
    public void onBackPressed() {
        if (gameStarted) {
            nativeChatClose();
            return;
        }
        super.onBackPressed();
    }
}
