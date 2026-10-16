package org.solosoul.rf312observer;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.UiAutomation;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.util.Xml;
import android.view.InputDevice;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import org.json.JSONObject;
import org.xmlpull.v1.XmlSerializer;
import java.io.StringWriter;

/** Independent public-fixture observer; never instruments the application under test. */
public final class Observer extends Instrumentation {
    private static final String APP = "com.solosoul.app";
    private static final String PICKER = "com.android.documentsui";
    private static final int MAX_NODES = 5000;
    private String token;
    private int visited;

    @Override public void onCreate(Bundle args) {
        super.onCreate(args);
        token = args.getString("token", "");
        if (!token.matches("[a-f0-9]{32}")) throw new IllegalArgumentException("Invalid namespace");
        start();
    }

    private static void write(File file, String value) throws Exception {
        try (FileOutputStream out = new FileOutputStream(file)) {
            out.write(value.getBytes(StandardCharsets.UTF_8));
        }
    }
    private static boolean allowed(String pkg) { return APP.equals(pkg) || PICKER.equals(pkg); }
    private static String text(CharSequence value) { return value == null ? "" : value.toString(); }
    private static String bounds(AccessibilityNodeInfo node) {
        Rect r = new Rect(); node.getBoundsInScreen(r);
        return "["+r.left+","+r.top+"]["+r.right+","+r.bottom+"]";
    }
    private void guardDepth(int depth) {
        if (depth > 200 || ++visited > MAX_NODES) throw new IllegalStateException("Hierarchy exceeds bound");
    }
    private void emit(XmlSerializer xml, AccessibilityNodeInfo node, int depth) throws Exception {
        guardDepth(depth);
        xml.startTag(null,"node");
        xml.attribute(null,"package",text(node.getPackageName()));
        xml.attribute(null,"class",text(node.getClassName()));
        xml.attribute(null,"text",text(node.getText()));
        xml.attribute(null,"content-desc",text(node.getContentDescription()));
        xml.attribute(null,"password",String.valueOf(node.isPassword()));
        xml.attribute(null,"enabled",String.valueOf(node.isEnabled()));
        xml.attribute(null,"clickable",String.valueOf(node.isClickable()));
        xml.attribute(null,"focusable",String.valueOf(node.isFocusable()));
        xml.attribute(null,"focused",String.valueOf(node.isFocused()));
        xml.attribute(null,"scrollable",String.valueOf(node.isScrollable()));
        xml.attribute(null,"view-id",text(node.getViewIdResourceName()));
        xml.attribute(null,"visible-to-user",String.valueOf(node.isVisibleToUser()));
        xml.attribute(null,"bounds",bounds(node));
        for (int i=0;i<node.getChildCount();i++) {
            AccessibilityNodeInfo child=node.getChild(i);
            if (child!=null) { try { emit(xml,child,depth+1); } finally { child.recycle(); } }
        }
        xml.endTag(null,"node");
    }
    private String hierarchy(AccessibilityNodeInfo root) throws Exception {
        StringWriter buffer=new StringWriter();
        XmlSerializer xml=Xml.newSerializer();xml.setOutput(buffer);
        xml.startDocument("UTF-8",true);xml.startTag(null,"hierarchy");
        if (root!=null && allowed(text(root.getPackageName()))) {
            visited=0;emit(xml,root,0);
        }
        xml.endTag(null,"hierarchy");xml.endDocument();return buffer.toString();
    }
    private static boolean matches(AccessibilityNodeInfo node,JSONObject target) throws Exception {
        return text(node.getPackageName()).equals(target.getString("package"))
            && text(node.getClassName()).equals(target.getString("class"))
            && text(node.getText()).equals(target.getString("text"))
            && text(node.getContentDescription()).equals(target.getString("content-desc"))
            && String.valueOf(node.isPassword()).equals(target.getString("password"))
            && bounds(node).equals(target.getString("bounds"));
    }
    private void find(AccessibilityNodeInfo node,JSONObject target,int depth,
                      ArrayList<AccessibilityNodeInfo> found) throws Exception {
        guardDepth(depth);
        if (matches(node,target)) found.add(AccessibilityNodeInfo.obtain(node));
        for (int i=0;i<node.getChildCount();i++) {
            AccessibilityNodeInfo child=node.getChild(i);
            if (child!=null) { try { find(child,target,depth+1,found); } finally { child.recycle(); } }
        }
    }
    private void inputCandidates(AccessibilityNodeInfo node,JSONObject target,int depth,
                                 org.json.JSONArray rows) throws Exception {
        guardDepth(depth);
        if (text(node.getPackageName()).equals(target.getString("package"))
            && text(node.getClassName()).equals(target.getString("class")) && rows.length()<16) {
            rows.put(new JSONObject().put("bounds",bounds(node)).put("nativeIdentity",node.hashCode())
                .put("windowIdentity",node.getWindowId()).put("password",node.isPassword())
                .put("passwordMatches",String.valueOf(node.isPassword()).equals(target.getString("password")))
                .put("textLength",text(node.getText()).length())
                .put("textMatches",text(node.getText()).equals(target.getString("text")))
                .put("descriptionLength",text(node.getContentDescription()).length())
                .put("descriptionMatches",text(node.getContentDescription()).equals(target.getString("content-desc")))
                .put("focused",node.isFocused()).put("focusable",node.isFocusable())
                .put("visible",node.isVisibleToUser()).put("enabled",node.isEnabled()));
        }
        for (int i=0;i<node.getChildCount();i++) {
            AccessibilityNodeInfo child=node.getChild(i);
            if (child!=null) { try { inputCandidates(child,target,depth+1,rows); } finally { child.recycle(); } }
        }
    }
    private AccessibilityNodeInfo exactTarget(AccessibilityNodeInfo root,JSONObject request) throws Exception {
        JSONObject target=request.getJSONObject("target");
        if (!allowed(target.getString("package")) ||
            !text(root.getPackageName()).equals(target.getString("package")))
            throw new IllegalStateException("Root and requested target must belong to one allowed package");
        ArrayList<AccessibilityNodeInfo> found=new ArrayList<>();
        visited=0;find(root,target,0,found);
        if (found.size()!=1) {
            for (AccessibilityNodeInfo node:found) node.recycle();
            org.json.JSONArray rows=new org.json.JSONArray();visited=0;
            inputCandidates(root,target,0,rows);
            JSONObject diagnostic=new JSONObject().put("exactMatchCount",found.size()).put("inputDispatched",false)
                .put("requestedBounds",target.getString("bounds")).put("requestedTextLength",target.getString("text").length())
                .put("requestedDescriptionLength",target.getString("content-desc").length()).put("candidates",rows);
            throw new IllegalStateException("Target must match exactly once; diagnostic="+diagnostic.toString());
        }
        AccessibilityNodeInfo node=found.get(0);
        if (!node.isEnabled() || !node.isVisibleToUser()) {
            node.recycle();throw new IllegalStateException("Target not visible and enabled");
        }
        return node;
    }
    private static boolean sameLogicalTarget(AccessibilityNodeInfo node,JSONObject target) throws Exception {
        return text(node.getPackageName()).equals(target.getString("package"))
            && text(node.getClassName()).equals(target.getString("class"))
            && text(node.getText()).equals(target.getString("text"))
            && text(node.getContentDescription()).equals(target.getString("content-desc"))
            && String.valueOf(node.isPassword()).equals(target.getString("password"));
    }
    private void compatible(AccessibilityNodeInfo node,JSONObject target,int depth,
                            org.json.JSONArray boxes,int[] count) throws Exception {
        guardDepth(depth);
        if (sameLogicalTarget(node,target)) { count[0]++;if (boxes.length()<16) boxes.put(bounds(node)); }
        for (int i=0;i<node.getChildCount();i++) {
            AccessibilityNodeInfo child=node.getChild(i);
            if (child!=null) { try { compatible(child,target,depth+1,boxes,count); } finally { child.recycle(); } }
        }
    }
    private static void finger(UiAutomation automation,float x,float y) {
        MotionEvent.PointerProperties properties=new MotionEvent.PointerProperties();
        properties.id=0;properties.toolType=MotionEvent.TOOL_TYPE_FINGER;
        MotionEvent.PointerCoords coords=new MotionEvent.PointerCoords();
        coords.x=x;coords.y=y;coords.size=1;
        long down=SystemClock.uptimeMillis();
        boolean accepted=true;
        for (int action:new int[]{MotionEvent.ACTION_DOWN,MotionEvent.ACTION_UP}) {
            coords.pressure=action==MotionEvent.ACTION_DOWN?1:0;
            MotionEvent event=MotionEvent.obtain(down,SystemClock.uptimeMillis(),action,1,
                new MotionEvent.PointerProperties[]{properties},new MotionEvent.PointerCoords[]{coords},
                0,0,1,1,0,0,InputDevice.SOURCE_TOUCHSCREEN,0);
            try { accepted=automation.injectInputEvent(event,true) && accepted; }
            finally { event.recycle(); }
        }
        if (!accepted) throw new IllegalStateException("Single native finger pair was not accepted");
    }
    private static void scrollFinger(UiAutomation automation,Rect area,boolean forward) {
        MotionEvent.PointerProperties properties=new MotionEvent.PointerProperties();
        properties.id=0;properties.toolType=MotionEvent.TOOL_TYPE_FINGER;
        MotionEvent.PointerCoords coords=new MotionEvent.PointerCoords();coords.size=1;coords.x=area.exactCenterX();
        float from=area.top+area.height()*(forward?.78f:.30f),to=area.top+area.height()*(forward?.30f:.78f);
        long down=SystemClock.uptimeMillis();boolean accepted=true;
        for (int i=0;i<=13;i++) {
            int action=i==0?MotionEvent.ACTION_DOWN:i==13?MotionEvent.ACTION_UP:MotionEvent.ACTION_MOVE;
            coords.y=from+(to-from)*Math.min(i,12)/12f;coords.pressure=i==13?0:1;
            MotionEvent event=MotionEvent.obtain(down,SystemClock.uptimeMillis(),action,1,
                new MotionEvent.PointerProperties[]{properties},new MotionEvent.PointerCoords[]{coords},
                0,0,1,1,0,0,InputDevice.SOURCE_TOUCHSCREEN,0);
            try { accepted=automation.injectInputEvent(event,true)&&accepted; } finally { event.recycle(); }
            if (i<12) SystemClock.sleep(30);
        }
        if (!accepted) throw new IllegalStateException("One bounded native scroll gesture refused");
    }
    private static boolean publicPickerTile(AccessibilityNodeInfo node,String filename) {
        return filename.matches("rf312-ocr-[a-f0-9]{32}\\.png") && PICKER.equals(text(node.getPackageName()))
            && "android.widget.LinearLayout".equals(text(node.getClassName())) && node.isFocusable()
            && !node.isPassword() && text(node.getText()).isEmpty()
            && text(node.getContentDescription()).startsWith(filename+", ");
    }
    private void pickerPreviewBoxes(AccessibilityNodeInfo node,String filename,int depth,ArrayList<Rect> boxes) {
        guardDepth(depth);
        if (PICKER.equals(text(node.getPackageName())) && text(node.getViewIdResourceName()).endsWith("/preview_icon")
            && text(node.getContentDescription()).equals("Preview the file "+filename)) {
            Rect box=new Rect();node.getBoundsInScreen(box);boxes.add(box);
        }
        for (int i=0;i<node.getChildCount();i++) {
            AccessibilityNodeInfo child=node.getChild(i);
            if (child!=null) { try { pickerPreviewBoxes(child,filename,depth+1,boxes); } finally { child.recycle(); } }
        }
    }
    private JSONObject perform(UiAutomation automation,JSONObject request,File rootDir,String name) throws Exception {
        JSONObject result=new JSONObject();
        String action=request.getString("action");
        result.put("action",action).put("nativeStartedAtNs",SystemClock.elapsedRealtimeNanos());
        AccessibilityNodeInfo root=automation.getRootInActiveWindow();
        try {
            String pkg=root==null?"":text(root.getPackageName());
            result.put("rootPackage",pkg).put("allowedRoot",allowed(pkg));
            if ("snapshot".equals(action)) {
                result.put("xml",hierarchy(root));
                if (request.optBoolean("screenshot",false)) {
                    if (!allowed(pkg)) throw new IllegalStateException("Never capture unrelated screen");
                    Bitmap image=automation.takeScreenshot();
                    if (image==null) throw new IllegalStateException("Native screenshot unavailable");
                    try (FileOutputStream out=new FileOutputStream(new File(rootDir,"frame-"+name+".png"))) {
                        if (!image.compress(Bitmap.CompressFormat.PNG,100,out))
                            throw new IllegalStateException("PNG encoding failed");
                    } finally { image.recycle(); }
                    result.put("screenshot","frame-"+name+".png");
                }
            } else {
                if (!allowed(pkg) || !pkg.equals(request.getString("expectedPackage")))
                    throw new IllegalStateException("Unexpected foreground root; no input dispatched");
                if ("inspect-target".equals(action)) {
                    String intent=request.getString("intent");
                    if (!"tap".equals(intent) && !"type-public".equals(intent) && !"scroll".equals(intent) && !"pick-public-file".equals(intent))
                        throw new IllegalArgumentException("Bound input intent required");
                    JSONObject target=request.getJSONObject("target");
                    if (!pkg.equals(target.getString("package"))) throw new IllegalStateException("Unowned inspection target");
                    ArrayList<AccessibilityNodeInfo> found=new ArrayList<>();
                    visited=0;find(root,target,0,found);
                    result.put("intent",intent).put("matchCount",found.size()).put("matched",found.size()==1)
                        .put("inputDispatched",false);
                    AccessibilityNodeInfo one=found.size()==1?found.get(0):null;
                    result.put("targetEnabled",one!=null&&one.isEnabled()).put("targetVisible",one!=null&&one.isVisibleToUser())
                        .put("targetFocused",one!=null&&one.isFocused()).put("targetFocusable",one!=null&&one.isFocusable())
                        .put("targetClickable",one!=null&&one.isClickable()).put("targetScrollable",one!=null&&one.isScrollable());
                    if ("pick-public-file".equals(intent)) result.put("targetPickerFileTile",one!=null&&publicPickerTile(one,request.getString("fileName")));
                    for (AccessibilityNodeInfo item:found) item.recycle();
                    org.json.JSONArray boxes=new org.json.JSONArray();int[] count={0};
                    visited=0;compatible(root,target,0,boxes,count);
                    result.put("compatibleCount",count[0]).put("candidateBounds",boxes);
                    org.json.JSONArray ime=new org.json.JSONArray();
                    for (android.view.accessibility.AccessibilityWindowInfo window:automation.getWindows()) {
                        try {
                            if (window.getType()==android.view.accessibility.AccessibilityWindowInfo.TYPE_INPUT_METHOD) {
                                Rect rect=new Rect();window.getBoundsInScreen(rect);
                                if (!rect.isEmpty()) ime.put("["+rect.left+","+rect.top+"]["+rect.right+","+rect.bottom+"]");
                            }
                        } finally { window.recycle(); }
                    }
                    result.put("imeBounds",ime).put("imeWindowCount",ime.length()).put("imeVisible",ime.length()>0);
                } else if ("tap".equals(action) || "type-public".equals(action)) {
                    AccessibilityNodeInfo node=exactTarget(root,request);
                    try {
                        if ("tap".equals(action)) {
                            boolean input="android.widget.EditText".contentEquals(node.getClassName()) && node.isFocusable();
                            if (!node.isClickable() && !input) throw new IllegalStateException("Target not actionable");
                            Rect rect=new Rect();node.getBoundsInScreen(rect);
                            android.util.DisplayMetrics display=new android.util.DisplayMetrics();
                            android.hardware.display.DisplayManager displays=(android.hardware.display.DisplayManager)getContext().getSystemService(android.content.Context.DISPLAY_SERVICE);
                            android.view.Display actual=displays.getDisplay(android.view.Display.DEFAULT_DISPLAY);
                            if (actual==null) throw new IllegalStateException("Native display unavailable");
                            actual.getRealMetrics(display);
                            if (rect.left<0 || rect.top<0 || rect.right>display.widthPixels || rect.bottom>display.heightPixels || rect.isEmpty())
                                throw new IllegalStateException("Target outside native display");
                            finger(automation,rect.exactCenterX(),rect.exactCenterY());
                            result.put("singleFingerPair",true).put("toolType","finger").put("bounds",bounds(node));
                        } else {
                            String value=request.getString("text");
                            boolean password="perf-baseline-only-password".equals(value);
                            if ((!password && !"needle".equals(value)) || !APP.equals(pkg) ||
                                !"android.widget.EditText".contentEquals(node.getClassName()) ||
                                !node.isFocused() || !node.isFocusable() || node.isPassword()!=password ||
                                !text(node.getText()).isEmpty())
                                throw new IllegalStateException("Only empty focused public fixture inputs may be typed");
                            KeyEvent[] events=KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD).getEvents(value.toCharArray());
                            if (events==null || events.length==0) throw new IllegalStateException("No native public key events");
                            for (KeyEvent event:events)
                                if (!automation.injectInputEvent(event,true)) throw new IllegalStateException("Public key input refused");
                            result.put("publicTextOnly",true).put("keyEvents",events.length);
                        }
                    } finally { node.recycle(); }
                } else if ("pick-public-file".equals(action)) {
                    String filename=request.getString("fileName");
                    AccessibilityNodeInfo node=exactTarget(root,request);
                    try {
                        if (!publicPickerTile(node,filename)) throw new IllegalStateException("Exact owned public picker tile required");
                        Rect area=new Rect();node.getBoundsInScreen(area);
                        android.util.DisplayMetrics metrics=new android.util.DisplayMetrics();
                        android.hardware.display.DisplayManager manager=(android.hardware.display.DisplayManager)getContext().getSystemService(android.content.Context.DISPLAY_SERVICE);
                        android.view.Display display=manager.getDisplay(android.view.Display.DEFAULT_DISPLAY);
                        if (display==null) throw new IllegalStateException("Native display required");display.getRealMetrics(metrics);
                        if (area.left<0 || area.top<0 || area.right>metrics.widthPixels || area.bottom>metrics.heightPixels || area.isEmpty())
                            throw new IllegalStateException("Public picker tile outside display");
                        ArrayList<Rect> previews=new ArrayList<>();visited=0;pickerPreviewBoxes(node,filename,0,previews);
                        if (previews.size()!=1 || previews.get(0).contains((int)area.exactCenterX(),(int)area.exactCenterY()))
                            throw new IllegalStateException("Unique preview control must be excluded from file selection point");
                        finger(automation,area.exactCenterX(),area.exactCenterY());
                        result.put("publicPickerFileOnly",true).put("fileName",filename)
                            .put("singleFingerPair",true).put("toolType","finger").put("bounds",bounds(node))
                            .put("previewControlExcluded",true);
                    } finally { node.recycle(); }
                } else if ("scroll".equals(action)) {
                    String direction=request.getString("direction");
                    if (!"forward".equals(direction)&&!"backward".equals(direction)) throw new IllegalArgumentException("Bound scroll direction required");
                    AccessibilityNodeInfo node=exactTarget(root,request);
                    try {
                        if (!node.isScrollable()) throw new IllegalStateException("Native scrollable target required");
                        Rect area=new Rect();node.getBoundsInScreen(area);
                        android.util.DisplayMetrics display=new android.util.DisplayMetrics();
                        android.hardware.display.DisplayManager manager=(android.hardware.display.DisplayManager)getContext().getSystemService(android.content.Context.DISPLAY_SERVICE);
                        android.view.Display actual=manager.getDisplay(android.view.Display.DEFAULT_DISPLAY);
                        if (actual==null) throw new IllegalStateException("Native display required");actual.getRealMetrics(display);
                        if (!area.intersect(0,0,display.widthPixels,display.heightPixels) || area.width()<100 || area.height()<300)
                            throw new IllegalStateException("Bounded visible scrolling area required");
                        scrollFinger(automation,area,"forward".equals(direction));
                        result.put("singleFingerGesture",true).put("toolType","finger").put("events",14)
                            .put("direction",direction).put("bounds",bounds(node));
                    } finally { node.recycle(); }
                } else if ("back".equals(action)) {
                    long now=SystemClock.uptimeMillis();
                    for (int kind:new int[]{KeyEvent.ACTION_DOWN,KeyEvent.ACTION_UP})
                        if (!automation.injectInputEvent(new KeyEvent(now,SystemClock.uptimeMillis(),kind,KeyEvent.KEYCODE_BACK,0),true))
                            throw new IllegalStateException("Back input refused");
                } else throw new IllegalArgumentException("Unknown bounded action");
            }
            result.put("success",true).put("nativeFinishedAtNs",SystemClock.elapsedRealtimeNanos());
            return result;
        } finally { if (root!=null) root.recycle(); }
    }
    @Override public void onStart() {
        File directory=new File(getTargetContext().getFilesDir(),token);
        try {
            if (!directory.mkdir()) throw new IllegalStateException("Namespace already exists");
            UiAutomation automation=getUiAutomation();
            if (automation==null) throw new IllegalStateException("Observer not connected");
            android.accessibilityservice.AccessibilityServiceInfo info=automation.getServiceInfo();
            if (info==null) throw new IllegalStateException("Observer service unavailable");
            info.flags|=android.accessibilityservice.AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS | android.accessibilityservice.AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS | android.accessibilityservice.AccessibilityServiceInfo.FLAG_INCLUDE_NOT_IMPORTANT_VIEWS;
            automation.setServiceInfo(info);
            if ((automation.getServiceInfo().flags&android.accessibilityservice.AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS)==0)
                throw new IllegalStateException("Native IME window geometry unavailable");
            write(new File(directory,"ready"),"ready");
            long deadline=SystemClock.uptimeMillis()+240000;
            for (int ordinal=1;SystemClock.uptimeMillis()<deadline && ordinal<=512;) {
                if (new File(directory,"stop").exists()) {
                    write(new File(directory,"stopped"),"stopped");finish(Activity.RESULT_OK,new Bundle());return;
                }
                String name=String.format(java.util.Locale.ROOT,"%03d",ordinal);
                File input=new File(directory,"request-"+name+".json");
                if (!input.exists()) { SystemClock.sleep(25);continue; }
                if (input.length()<=0 || input.length()>16384) throw new IllegalStateException("Invalid bounded request");
                JSONObject request=new JSONObject(new String(Files.readAllBytes(input.toPath()),StandardCharsets.UTF_8));
                JSONObject result=perform(automation,request,directory,name);
                File temp=new File(directory,"response-"+name+".tmp");
                write(temp,result.toString());
                if (!temp.renameTo(new File(directory,"response-"+name+".json")))
                    throw new IllegalStateException("Atomic response publication failed");
                ordinal++;
            }
            throw new IllegalStateException("Full journey observer bound exhausted");
        } catch (Exception error) {
            try { write(new File(directory,"error"),error.toString()); } catch (Exception ignored) {}
            Bundle failure=new Bundle();failure.putString("error",error.toString());finish(Activity.RESULT_CANCELED,failure);
        }
    }
}
