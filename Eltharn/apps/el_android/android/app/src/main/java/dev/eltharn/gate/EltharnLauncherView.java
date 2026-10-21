package dev.eltharn.gate;

import android.content.Context;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AtomicFile;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.text.InputFilter;
import android.text.InputType;
import android.view.Gravity;
import android.view.inputmethod.EditorInfo;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.AdapterView;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;
import android.widget.SeekBar;
import android.widget.Toast;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;


final class EltharnLauncherView extends ScrollView {
    private static final int WHITE = Color.rgb(245, 246, 250);
    private static final int MUTED = Color.rgb(169, 173, 184);
    private static final int GREEN = Color.rgb(79, 220, 91);
    private static final int PANEL = Color.rgb(32, 35, 43);
    private static final int INSET = Color.rgb(20, 22, 28);

    private TextView runtimeStatus;
    private Button launchButton;
    private final Spinner accountSpinner;
    private final ArrayAdapter<String> accountAdapter;
    private final List<String> accountNames = new ArrayList<>();
    private final SharedPreferences preferences;
    private String selectedControls = "classic";
    private JSONObject touchProfile;
    private TouchPreviewView touchPreview;
    private boolean touchSettingsInitializing;
    private static native String nativeTouchProfile(String value);
    private static native String nativeTouchPreview(String value, int width, int height);
    private String selectedOrientation = "landscape";
    private String activeAccountName = "";
    private boolean launchInProgress;

    EltharnLauncherView(Context context) {
        super(context);
        preferences = context.getSharedPreferences("eltharn_launcher", Context.MODE_PRIVATE);
        selectedControls = preferences.getString("touch_controls", "classic");
        selectedOrientation = preferences.getString("orientation", "landscape");
        touchProfile = loadTouchProfile();
        setFillViewport(true);
        setBackground(gradientBackground());
        setClipToPadding(false);
        setPadding(dp(18), dp(22), dp(18), dp(28));


        setFocusableInTouchMode(true);
        requestFocus();

        LinearLayout page = column();
        addView(page, new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT));

        page.addView(title("Eltharn", 28, WHITE, true));
        page.addView(space(18));

        Button controlsToggle = new Button(context);
        controlsToggle.setText("Customize touch controls");
        controlsToggle.setTextColor(WHITE);
        controlsToggle.setTextSize(13);
        controlsToggle.setAllCaps(false);
        controlsToggle.setBackground(round(Color.rgb(52, 57, 69), 8));
        page.addView(controlsToggle, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(46)));
        page.addView(space(10));
        LinearLayout controls = panel(PANEL, 12);
        controls.addView(text("Touch layout", 13, WHITE, true));
        Spinner controlsSpinner = new Spinner(context);
        ArrayAdapter<String> controlAdapter = new ArrayAdapter<>(context,
                android.R.layout.simple_spinner_item, new String[]{"Classic", "Compact", "Off"});
        controlAdapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        controlsSpinner.setAdapter(controlAdapter);
        controlsSpinner.setSelection("compact".equals(selectedControls) ? 1 : ("off".equals(selectedControls) ? 2 : 0));
        LinearLayout touchSettings = column();
        controlsSpinner.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            private boolean initialized;
            @Override
            public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                if (!initialized) { initialized = true; return; }
                selectedControls = position == 1 ? "compact" : (position == 2 ? "off" : "classic");
                try {
                    touchProfile = new JSONObject(nativeTouchProfile(selectedControls));
                    rebuildTouchSettings(touchSettings);
                    persistTouchProfile();
                } catch (JSONException error) {
                    Toast.makeText(getContext(), "Touch preset could not be loaded", Toast.LENGTH_LONG).show();
                }
            }
            @Override public void onNothingSelected(AdapterView<?> parent) {}
        });
        controls.addView(controlsSpinner, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(48)));
        touchPreview = new TouchPreviewView(context);
        controls.addView(touchPreview, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(210)));
        controls.addView(touchSettings);
        rebuildTouchSettings(touchSettings);
        controls.addView(space(8));
        controls.addView(text("Screen orientation", 13, WHITE, true));
        Spinner orientationSpinner = new Spinner(context);
        String[] orientationLabels = new String[]{"Landscape", "Auto-rotate"};
        ArrayAdapter<String> orientationAdapter = new ArrayAdapter<String>(
                context, android.R.layout.simple_spinner_item, orientationLabels);
        orientationAdapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        orientationSpinner.setAdapter(orientationAdapter);
        orientationSpinner.setSelection("auto".equals(selectedOrientation) ? 1 : 0);
        orientationSpinner.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            @Override
            public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                selectedOrientation = position == 1 ? "auto" : "landscape";
                preferences.edit().putString("orientation", selectedOrientation).apply();
            }

            @Override
            public void onNothingSelected(AdapterView<?> parent) {

            }
        });
        controls.addView(orientationSpinner, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(48)));
        controls.setVisibility(View.GONE);
        controlsToggle.setOnClickListener(v -> {
            boolean expanded = controls.getVisibility() != View.VISIBLE;
            controls.setVisibility(expanded ? View.VISIBLE : View.GONE);
            controlsToggle.setText(expanded ? "Hide touch controls" : "Customize touch controls");
        });
        page.addView(controls);
        page.addView(space(12));

        page.addView(text("ACCOUNT", 11, MUTED, true));
        page.addView(space(7));
        LinearLayout account = panel(PANEL, 12);
        loadAccounts();
        accountSpinner = new Spinner(context);
        accountAdapter = new ArrayAdapter<String>(context, android.R.layout.simple_spinner_item, accountNames);
        accountAdapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        accountSpinner.setAdapter(accountAdapter);
        accountSpinner.setVisibility(accountNames.isEmpty() ? View.GONE : View.VISIBLE);
        accountSpinner.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            @Override
            public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                if (accountNames.isEmpty()) {
                    return;
                }
                preferences.edit().putInt("active_account", position).apply();
                updateAccount(accountNames.get(position));
            }

            @Override
            public void onNothingSelected(AdapterView<?> parent) {

            }
        });
        if (!accountNames.isEmpty()) {
            int active = Math.max(0, Math.min(
                    preferences.getInt("active_account", 0), accountNames.size() - 1));
            accountSpinner.setSelection(active);
            updateAccount(accountNames.get(active));
        }
        account.addView(accountSpinner, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(48)));
        LinearLayout addAccountRow = new LinearLayout(context);
        addAccountRow.setGravity(Gravity.CENTER_VERTICAL);
        EditText nickname = new EditText(context);
        nickname.setHint("Nickname");
        nickname.setHintTextColor(Color.rgb(125, 130, 143));
        nickname.setTextColor(WHITE);
        nickname.setSingleLine(true);
        nickname.setTextSize(14);
        nickname.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        nickname.setImeOptions(EditorInfo.IME_ACTION_DONE);
        nickname.setFilters(new InputFilter[]{new InputFilter.LengthFilter(16)});
        nickname.setPadding(dp(10), 0, dp(8), 0);
        nickname.setBackground(round(INSET, 8));
        addAccountRow.addView(nickname, new LinearLayout.LayoutParams(0, dp(48), 1));
        Button addAccount = new Button(context);
        addAccount.setText("Add");
        addAccount.setTextColor(WHITE);
        addAccount.setTextSize(12);
        addAccount.setAllCaps(false);
        addAccount.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        addAccount.setBackground(round(Color.rgb(52, 57, 69), 8));
        LinearLayout.LayoutParams addButtonParams = new LinearLayout.LayoutParams(dp(112), dp(48));
        addButtonParams.setMargins(dp(8), 0, 0, 0);
        addAccountRow.addView(addAccount, addButtonParams);
        addAccount.setOnClickListener(v -> addAccount(nickname));
        nickname.setOnEditorActionListener((view, actionId, event) -> {
            if (actionId == EditorInfo.IME_ACTION_DONE) {
                addAccount(nickname);
                return true;
            }
            return false;
        });
        account.addView(addAccountRow, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(48)));
        page.addView(account);
        page.addView(space(16));

        launchButton = new Button(context);
        launchButton.setText("Launch Eltharn");
        launchButton.setTextColor(Color.rgb(10, 22, 12));
        launchButton.setTextSize(14);
        launchButton.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        launchButton.setAllCaps(false);
        launchButton.setOnClickListener(v -> {
            if (getContext() instanceof EltharnGateActivity activity) {
                activity.requestGameStart(
                        "1.21.11", activeAccountName, touchProfile.toString(), selectedOrientation);
            } else {
                showRuntimeStatus();
            }
        });
        launchButton.setBackground(round(GREEN, 10));
        page.addView(launchButton, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(50)));

        runtimeStatus = text("Starting game…", 11, MUTED, false);
        runtimeStatus.setPadding(0, dp(12), 0, 0);
        runtimeStatus.setVisibility(View.GONE);
        page.addView(runtimeStatus);
        updateLaunchAvailability();
        setDescendantFocusability(ViewGroup.FOCUS_BEFORE_DESCENDANTS);
        requestFocus();
    }

    private void showRuntimeStatus() {
        runtimeStatus.setText("Starting game…");
        runtimeStatus.setVisibility(View.VISIBLE);
    }

    void setLaunchInProgress(boolean value) {
        launchInProgress = value;
        updateLaunchAvailability();
        if (value) {
            showRuntimeStatus();
        } else {
            runtimeStatus.setVisibility(View.GONE);
        }
    }

    private void loadAccounts() {
        String encoded = preferences.getString("accounts", "[]");
        try {
            JSONArray values = new JSONArray(encoded);
            for (int i = 0; i < values.length(); i++) {
                String name = values.optString(i, "").trim();
                if (!name.isEmpty() && !accountNames.contains(name)) {
                    accountNames.add(name);
                }
            }
        } catch (JSONException ignored) {

        }
    }

    private void addAccount(EditText nickname) {
        String name = nickname.getText().toString().trim();
        if (name.isEmpty()) {
            nickname.setError("Please enter a nickname");
            return;
        }
        if (accountNames.contains(name)) {
            nickname.setError("This nickname is already added");
            return;
        }
        accountNames.add(name);
        accountAdapter.notifyDataSetChanged();
        accountSpinner.setVisibility(View.VISIBLE);
        int index = accountNames.size() - 1;
        accountSpinner.setSelection(index);
        preferences.edit().putInt("active_account", index).apply();
        saveAccounts();
        nickname.setText("");
        updateAccount(name);
        Toast.makeText(getContext(), "Account " + name + " saved", Toast.LENGTH_SHORT).show();
    }

    private void saveAccounts() {
        JSONArray values = new JSONArray();
        for (String name : accountNames) {
            values.put(name);
        }
        preferences.edit().putString("accounts", values.toString()).apply();
    }

    private void updateAccount(String name) {
        activeAccountName = name;
        updateLaunchAvailability();
    }

    private void updateLaunchAvailability() {
        if (launchButton == null) {
            return;
        }
        boolean supported = !activeAccountName.trim().isEmpty();
        launchButton.setEnabled(supported && !launchInProgress);
        launchButton.setAlpha(supported && !launchInProgress ? 1.0f : 0.55f);
    }

    private LinearLayout column() {
        LinearLayout layout = new LinearLayout(getContext());
        layout.setOrientation(LinearLayout.VERTICAL);
        return layout;
    }

    private LinearLayout panel(int color, int radius) {
        LinearLayout layout = column();
        layout.setPadding(dp(14), dp(12), dp(14), dp(12));
        layout.setBackground(round(color, radius));
        return layout;
    }

    private TextView text(String value, int size, int color, boolean bold) {
        TextView view = new TextView(getContext());
        view.setText(value);
        view.setTextColor(color);
        view.setTextSize(size);
        view.setTypeface(Typeface.DEFAULT, bold ? Typeface.BOLD : Typeface.NORMAL);
        return view;
    }

    private TextView title(String value, int size, int color, boolean bold) {
        TextView view = text(value, size, color, bold);
        view.setGravity(Gravity.START);
        return view;
    }

    private View space(int heightDp) {
        View view = new View(getContext());
        view.setLayoutParams(new LinearLayout.LayoutParams(1, dp(heightDp)));
        return view;
    }

    private GradientDrawable round(int color, int radiusDp) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(color);
        drawable.setCornerRadius(dp(radiusDp));
        drawable.setStroke(dp(1), Color.argb(35, 255, 255, 255));
        return drawable;
    }

    private GradientDrawable gradientBackground() {
        GradientDrawable drawable = new GradientDrawable(
                GradientDrawable.Orientation.TL_BR,
                new int[]{Color.rgb(20, 24, 34), Color.rgb(12, 14, 20), Color.rgb(19, 18, 25)});
        return drawable;
    }

    private File touchProfileFile() {
        File base = getContext().getExternalFilesDir(null);
        if (base == null) base = getContext().getFilesDir();
        return new File(base, "eltharn/game/touch-controls.json");
    }

    private JSONObject loadTouchProfile() {
        String stored = preferences.getString("touch_controls_profile", selectedControls);
        File file = touchProfileFile();
        if (file.isFile() || new File(file.getPath() + ".bak").isFile()) {
            try (FileInputStream input = new AtomicFile(file).openRead()) {
                byte[] bytes = new byte[16385];
                int count = 0;
                while (count < bytes.length) {
                    int read = input.read(bytes, count, bytes.length - count);
                    if (read < 0) break;
                    count += read;
                }
                if (count > 16384) throw new IOException("Profile exceeds 16 KiB");
                stored = new String(bytes, 0, count, StandardCharsets.UTF_8);
            } catch (IOException error) {
                Toast.makeText(getContext(), "Saved touch profile could not be read", Toast.LENGTH_LONG).show();
            }
        }
        try {
            return new JSONObject(nativeTouchProfile(stored));
        } catch (JSONException error) {
            Toast.makeText(getContext(), "Invalid touch profile. Choose a preset to replace it.", Toast.LENGTH_LONG).show();
            try {
                return new JSONObject(nativeTouchProfile(selectedControls));
            } catch (JSONException unavailable) {
                throw new IllegalStateException("Shared touch presets are unavailable", unavailable);
            }
        }
    }

    private void persistTouchProfile() {
        if (touchSettingsInitializing) return;
        String canonical = nativeTouchProfile(touchProfile.toString());
        if (canonical == null || canonical.isEmpty()) {
            Toast.makeText(getContext(), "Touch layout is invalid", Toast.LENGTH_LONG).show();
            return;
        }
        File file = touchProfileFile();
        File parent = file.getParentFile();
        if (parent == null || (!parent.isDirectory() && !parent.mkdirs())) {
            Toast.makeText(getContext(), "Touch settings folder is unavailable", Toast.LENGTH_LONG).show();
            return;
        }
        AtomicFile target = new AtomicFile(file);
        FileOutputStream output = null;
        try {
            output = target.startWrite();
            output.write(canonical.getBytes(StandardCharsets.UTF_8));
            target.finishWrite(output);
            preferences.edit().putString("touch_controls_profile", canonical)
                    .putString("touch_controls", selectedControls).apply();
        } catch (IOException error) {
            if (output != null) target.failWrite(output);
            Toast.makeText(getContext(), "Touch settings could not be saved", Toast.LENGTH_LONG).show();
        }
        if (touchPreview != null) touchPreview.invalidate();
    }

    private interface TouchFloatSetter { void set(float value) throws JSONException; }

    private void touchSlider(LinearLayout parent, String label, int min, int max,
                             float current, TouchFloatSetter setter) {
        TextView valueLabel = text(label + "  " + Math.round(current * 100) + "%", 12, MUTED, false);
        parent.addView(valueLabel);
        SeekBar slider = new SeekBar(getContext());
        slider.setMax(max - min);
        slider.setProgress(Math.max(0, Math.min(max - min, Math.round(current * 100) - min)));
        slider.setProgressTintList(android.content.res.ColorStateList.valueOf(GREEN));
        slider.setThumbTintList(android.content.res.ColorStateList.valueOf(GREEN));
        slider.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
            @Override public void onProgressChanged(SeekBar view, int progress, boolean fromUser) {
                if (!fromUser || touchSettingsInitializing) return;
                float value = (progress + min) / 100.0f;
                try {
                    setter.set(value);
                    valueLabel.setText(label + "  " + (progress + min) + "%");
                    if (touchPreview != null) touchPreview.invalidate();
                } catch (JSONException error) {
                    Toast.makeText(getContext(), "Touch setting could not be changed", Toast.LENGTH_SHORT).show();
                }
            }
            @Override public void onStartTrackingTouch(SeekBar view) {}
            @Override public void onStopTrackingTouch(SeekBar view) { persistTouchProfile(); }
        });
        parent.addView(slider, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(38)));
    }

    private void rebuildTouchSettings(LinearLayout panel) {
        touchSettingsInitializing = true;
        panel.removeAllViews();
        android.widget.Switch enabled = new android.widget.Switch(getContext());
        enabled.setText("Show touch controls");
        enabled.setTextColor(WHITE);
        enabled.setChecked(touchProfile.optBoolean("enabled", true));
        enabled.setOnCheckedChangeListener((button, checked) -> {
            try { touchProfile.put("enabled", checked); persistTouchProfile(); }
            catch (JSONException error) { throw new IllegalStateException(error); }
        });
        panel.addView(enabled);
        android.widget.Switch mirror = new android.widget.Switch(getContext());
        mirror.setText("Left-handed layout");
        mirror.setTextColor(WHITE);
        mirror.setChecked("left".equals(touchProfile.optString("handedness")));
        mirror.setOnCheckedChangeListener((button, checked) -> {
            try { touchProfile.put("handedness", checked ? "left" : "right"); persistTouchProfile(); }
            catch (JSONException error) { throw new IllegalStateException(error); }
        });
        panel.addView(mirror);
        touchSlider(panel, "Control size", 60, 150, (float) touchProfile.optDouble("scale", 1.0), value -> touchProfile.put("scale", value));
        touchSlider(panel, "Opacity", 15, 100, (float) touchProfile.optDouble("opacity", 0.72), value -> touchProfile.put("opacity", value));
        touchSlider(panel, "Movement dead zone", 5, 60, (float) touchProfile.optDouble("dead_zone", 0.2), value -> touchProfile.put("dead_zone", value));
        touchSlider(panel, "Look sensitivity", 25, 300, (float) touchProfile.optDouble("look_sensitivity", 1.0), value -> touchProfile.put("look_sensitivity", value));
        try {
            JSONObject joystick = touchProfile.getJSONObject("joystick");
            JSONArray center = joystick.getJSONArray("center");
            touchSlider(panel, "Joystick horizontal position", 0, 100, (float) center.getDouble(0), value -> center.put(0, value));
            touchSlider(panel, "Joystick vertical position", 0, 100, (float) center.getDouble(1), value -> center.put(1, value));
            touchSlider(panel, "Joystick radius", 3, 25, (float) joystick.getDouble("radius"), value -> joystick.put("radius", value));
            JSONArray buttons = touchProfile.getJSONArray("buttons");
            panel.addView(text("Action buttons", 13, WHITE, true));
            Spinner actions = new Spinner(getContext());
            List<String> labels = new ArrayList<>();
            for (int i = 0; i < buttons.length(); i++) {
                labels.add(buttons.getJSONObject(i).getString("action").replace('_', ' '));
            }
            ArrayAdapter<String> adapter = new ArrayAdapter<>(getContext(), android.R.layout.simple_spinner_item, labels);
            adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
            actions.setAdapter(adapter);
            LinearLayout buttonEditor = column();
            actions.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
                @Override public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                    buttonEditor.removeAllViews();
                    try {
                        JSONObject button = buttons.getJSONObject(position);
                        JSONObject placement = button.getJSONObject("placement");
                        JSONArray buttonCenter = placement.getJSONArray("center");
                        android.widget.Switch visible = new android.widget.Switch(getContext());
                        visible.setText("Show this button");
                        visible.setTextColor(WHITE);
                        visible.setChecked(button.optBoolean("visible", true));
                        visible.setOnCheckedChangeListener((source, checked) -> {
                            try { button.put("visible", checked); persistTouchProfile(); }
                            catch (JSONException error) { throw new IllegalStateException(error); }
                        });
                        buttonEditor.addView(visible);
                        touchSlider(buttonEditor, "Horizontal position", 0, 100, (float) buttonCenter.getDouble(0), value -> buttonCenter.put(0, value));
                        touchSlider(buttonEditor, "Vertical position", 0, 100, (float) buttonCenter.getDouble(1), value -> buttonCenter.put(1, value));
                        touchSlider(buttonEditor, "Button radius", 3, 25, (float) placement.getDouble("radius"), value -> placement.put("radius", value));
                    } catch (JSONException error) { throw new IllegalStateException(error); }
                }
                @Override public void onNothingSelected(AdapterView<?> parent) {}
            });
            panel.addView(actions, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(44)));
            panel.addView(buttonEditor);
        } catch (JSONException error) { throw new IllegalStateException("Validated touch profile is incomplete", error); }
        touchSettingsInitializing = false;
        if (touchPreview != null) touchPreview.invalidate();
    }

    private final class TouchPreviewView extends View {
        private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        TouchPreviewView(Context context) { super(context); setContentDescription("Touch control layout preview"); }
        @Override protected void onDraw(Canvas canvas) {
            super.onDraw(canvas);
            paint.setStyle(Paint.Style.FILL);
            paint.setColor(Color.rgb(13, 23, 34));
            canvas.drawRoundRect(0, 0, getWidth(), getHeight(), dp(12), dp(12), paint);
            String json = nativeTouchPreview(touchProfile.toString(), getWidth(), getHeight());
            try {
                JSONObject layout = new JSONObject(json);
                if (!layout.getBoolean("enabled")) {
                    paint.setColor(MUTED); paint.setTextSize(dp(13));
                    canvas.drawText("Touch controls are off", dp(14), getHeight() / 2.0f, paint);
                    return;
                }
                float opacity = (float) layout.getDouble("opacity");
                JSONObject joystick = layout.getJSONObject("joystick");
                JSONArray center = joystick.getJSONArray("center");
                float x = (float) center.getDouble(0), y = (float) center.getDouble(1), r = (float) joystick.getDouble("radius");
                circle(canvas, x, y, r, Color.argb(Math.round(opacity * 110), 39, 65, 83));
                ring(canvas, x, y, r, Color.argb(Math.round(opacity * 200), 157, 194, 212), 1.2f);
                ring(canvas, x, y, r * 0.72f, Color.argb(Math.round(opacity * 80), 157, 194, 212), 1.0f);
                circle(canvas, x, y, r * 0.36f, Color.argb(Math.round(opacity * 235), 62, 105, 125));
                ring(canvas, x, y, r * 0.36f, Color.argb(Math.round(opacity * 240), 179, 222, 230), 1.2f);
                JSONArray buttons = layout.getJSONArray("buttons");
                for (int i = 0; i < buttons.length(); i++) {
                    JSONObject button = buttons.getJSONObject(i), shape = button.getJSONObject("circle");
                    JSONArray c = shape.getJSONArray("center");
                    float bx = (float) c.getDouble(0), by = (float) c.getDouble(1), br = (float) shape.getDouble("radius");
                    circle(canvas, bx, by, br, Color.argb(Math.round(opacity * 180), 27, 46, 62));
                    ring(canvas, bx, by, br, Color.argb(Math.round(opacity * 190), 155, 190, 211), 1.0f);
                    JSONArray glyph = button.getJSONArray("glyph");
                    paint.setStyle(Paint.Style.STROKE); paint.setStrokeCap(Paint.Cap.ROUND);
                    paint.setStrokeWidth(Math.max(1.1f, br * 0.09f));
                    paint.setColor(Color.argb(Math.round(opacity * 255), 225, 244, 250));
                    for (int segment = 0; segment < glyph.length(); segment++) {
                        JSONArray line = glyph.getJSONArray(segment);
                        canvas.drawLine(bx + (float) line.getDouble(0) * br * 0.62f, by + (float) line.getDouble(1) * br * 0.62f,
                                bx + (float) line.getDouble(2) * br * 0.62f, by + (float) line.getDouble(3) * br * 0.62f, paint);
                    }
                }
            } catch (JSONException error) {
                paint.setStyle(Paint.Style.FILL); paint.setColor(MUTED); paint.setTextSize(dp(12));
                canvas.drawText("Adjust the touch layout above", dp(14), getHeight() / 2.0f, paint);
            }
        }
        private void circle(Canvas canvas, float x, float y, float radius, int color) {
            paint.setStyle(Paint.Style.FILL); paint.setColor(color); canvas.drawCircle(x, y, radius, paint);
        }
        private void ring(Canvas canvas, float x, float y, float radius, int color, float width) {
            paint.setStyle(Paint.Style.STROKE); paint.setStrokeWidth(width); paint.setColor(color); canvas.drawCircle(x, y, radius, paint);
        }
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
