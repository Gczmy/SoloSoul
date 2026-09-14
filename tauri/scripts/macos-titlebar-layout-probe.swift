// 只验证离屏原生布局，不显示窗口、不访问用户数据、不声明通过视觉/恢复验收。
import AppKit
import WebKit

@MainActor
func inspect(fullBleed: Bool) {
    let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 1200, height: 800),
        styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
        backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    window.titlebarAppearsTransparent = true
    window.titleVisibility = .hidden
    window.isOpaque = false
    window.backgroundColor = .white.withAlphaComponent(0.001)
    let root = window.contentView!
    if #available(macOS 26, *) {
        let glass = NSGlassEffectView(frame: root.bounds)
        glass.cornerRadius = 16
        glass.autoresizingMask = [.width, .height]
        root.addSubview(glass)
    }
    let webview = WKWebView(frame: .zero)
    root.addSubview(webview)
    webview.translatesAutoresizingMaskIntoConstraints = false
    let target: Any = fullBleed ? root : window.contentLayoutGuide!
    NSLayoutConstraint.activate([NSLayoutConstraint.Attribute.left, .right, .top, .bottom].map {
        NSLayoutConstraint(item: webview, attribute: $0, relatedBy: .equal,
                           toItem: target, attribute: $0, multiplier: 1, constant: 0)
    })
    let originalWebview = ObjectIdentifier(webview)
    let originalButtons = [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton].map {
        ObjectIdentifier(window.standardWindowButton($0)!)
    }
    for size in [NSSize(width: 1200, height: 800), NSSize(width: 840, height: 620)] {
        window.setContentSize(size)
        root.layoutSubtreeIfNeeded()
        let frame = webview.frame
        let gap = root.bounds.maxY - frame.maxY
        let close = window.standardWindowButton(.closeButton)!
        let buttonFrame = root.convert(close.bounds, from: close)
        precondition(ObjectIdentifier(webview) == originalWebview)
        precondition(webview.superview === root)
        precondition(window.backgroundColor.alphaComponent == 0.001)
        precondition([NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton].map {
            ObjectIdentifier(window.standardWindowButton($0)!)
        } == originalButtons)
        if fullBleed {
            precondition(frame == root.bounds)
            precondition(frame.contains(buttonFrame))
        } else {
            precondition(gap > 0)
            precondition(!frame.contains(buttonFrame))
        }
        print("\(fullBleed ? "full-root" : "layout-guide") size=\(size) root=\(root.bounds) webview=\(frame) titlebarGap=\(gap) trafficLight=\(buttonFrame)")
    }
    window.close()
}

MainActor.assumeIsolated {
    let app = NSApplication.shared
    app.setActivationPolicy(.prohibited)
    inspect(fullBleed: false)
    inspect(fullBleed: true)
    print("PASS: full-root constraints cover the native titlebar without rebuilding glass/webview/buttons; visual restoration still requires integration testing")
}
