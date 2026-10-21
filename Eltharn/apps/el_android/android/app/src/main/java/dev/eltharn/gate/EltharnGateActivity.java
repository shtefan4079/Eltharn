package dev.eltharn.gate;

import android.content.Intent;
import android.app.ActivityManager;
import android.graphics.Color;
import android.os.Bundle;
import android.os.Process;
import android.widget.Toast;
import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.graphics.Insets;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowInsetsCompat;
import java.util.List;

public final class EltharnGateActivity extends AppCompatActivity {
    private EltharnLauncherView launcherView;
    private boolean launchInFlight;

    static {
        System.loadLibrary("eltharn_android");
    }

    private final ActivityResultLauncher<Intent> gameLauncher = registerForActivityResult(
            new ActivityResultContracts.StartActivityForResult(), result -> {
                launchInFlight = false;
                if (launcherView != null) {
                    launcherView.setLaunchInProgress(false);
                }
                Intent data = result.getData();
                String error = data == null ? null : data.getStringExtra("error");
                if (error != null && !error.isEmpty()) {
                    Toast.makeText(this, error, Toast.LENGTH_LONG).show();
                }
            });

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        launchInFlight = state != null && state.getBoolean("game_launch_in_flight");
        getWindow().setStatusBarColor(Color.rgb(14, 15, 19));
        getWindow().setNavigationBarColor(Color.rgb(14, 15, 19));
        EltharnLauncherView launcher = new EltharnLauncherView(this);
        launcherView = launcher;
        launcher.setLaunchInProgress(launchInFlight);
        int left = launcher.getPaddingLeft();
        int top = launcher.getPaddingTop();
        int right = launcher.getPaddingRight();
        int bottom = launcher.getPaddingBottom();
        ViewCompat.setOnApplyWindowInsetsListener(launcher, (view, insets) -> {
            Insets safe = insets.getInsets(
                    WindowInsetsCompat.Type.systemBars() | WindowInsetsCompat.Type.displayCutout());
            view.setPadding(left + safe.left, top + safe.top,
                    right + safe.right, bottom + safe.bottom);
            return insets;
        });
        setContentView(launcher);
        ViewCompat.requestApplyInsets(launcher);
    }

    void requestGameStart(String protocol, String username, String controls, String orientation) {
        if (launchInFlight) {
            return;
        }
        if (hasActiveGameProcess()) {
            Toast.makeText(this, "The previous game is still closing", Toast.LENGTH_SHORT).show();
            return;
        }
        launchInFlight = true;
        launcherView.setLaunchInProgress(true);
        Intent launch = new Intent(this, EltharnGateGameActivity.class);
        launch.putExtra("protocol", protocol);
        launch.putExtra("username", username);
        launch.putExtra("controls", controls);
        launch.putExtra("orientation", orientation);
        try {
            gameLauncher.launch(launch);
        } catch (RuntimeException error) {
            launchInFlight = false;
            launcherView.setLaunchInProgress(false);
            Toast.makeText(this, "Unable to start game activity", Toast.LENGTH_LONG).show();
        }
    }

    private boolean hasActiveGameProcess() {
        ActivityManager manager = (ActivityManager) getSystemService(ACTIVITY_SERVICE);
        if (manager == null) {
            return true;
        }
        List<ActivityManager.RunningAppProcessInfo> processes = manager.getRunningAppProcesses();
        if (processes == null) {
            return true;
        }
        String expected = getPackageName() + ":game";
        for (ActivityManager.RunningAppProcessInfo process : processes) {
            if (process.uid == Process.myUid() && expected.equals(process.processName)) {
                return true;
            }
        }
        return false;
    }

    @Override
    protected void onSaveInstanceState(Bundle state) {
        state.putBoolean("game_launch_in_flight", launchInFlight);
        super.onSaveInstanceState(state);
    }
}
