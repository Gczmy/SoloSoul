// 仅用于 macOS 26+ 图形会话诊断；不包含 Tauri、WebKit 或用户数据访问。
// xcrun swiftc -framework AppKit macos-native-appearance.swift -o /tmp/solosoul-native-appearance
// 用 ⌘1 / ⌘2 / ⌘3 切换材质，再分别通过 Dock / 台前调度恢复窗口。
// --variant=alpha|no-shadow|no-flatten|rehost|combined 选择独立实验；默认 baseline。
// --check-configurations 仅验证各实验的原生配置，不显示窗口，不证明零黑帧。
// 焦点回调仅打印事件，绝不修改外观；目测结果不能由布局断言替代。
// 2026-09-14 本机 macOS 26.6 人工对照：Liquid Glass 恢复时闪纯黑，
// 传统毛玻璃闪灰黑，不透明背景不闪。两种透明材质均可在无 Tauri/WebKit 时复现；
// 这定位到原生透明窗口路径，不足以断言 AppKit/WindowServer 内部的具体故障点。
import AppKit
import ObjectiveC

private enum WindowTrial: String, CaseIterable {
    case baseline, alpha, noShadow = "no-shadow", noFlatten = "no-flatten", rehost, combined

    var title: String {
        switch self {
        case .baseline: return "A · 原始配置"
        case .alpha: return "B · 微量背景色"
        case .noShadow: return "C · 关闭窗口阴影"
        case .noFlatten: return "D · 禁用自动扁平化"
        case .rehost: return "E · 禁用扁平化 + 初始化图层"
        case .combined: return "F · E + 微量背景色（保留阴影）"
        }
    }

    var backgroundAlpha: CGFloat { self == .alpha || self == .combined ? 0.001 : 0 }
    var disablesFlattening: Bool { [.noFlatten, .rehost, .combined].contains(self) }
    var rehostsLayers: Bool { self == .rehost || self == .combined }
}

// 实验用私有接口：先检查 getter/setter 的存在与 BOOL 签名，再调用对应实现。
// macOS 26.6 的扁平化方法带下划线；不依赖 KVC 的私有 setter 搜索规则。
// 不硬编码 CGS 窗口标志；不支持时将该实验标为失败，不伪装成已启用。
@MainActor
private func checkedBoolProperty(_ window: NSWindow, _ key: String, set value: Bool? = nil) throws -> Bool {
    func resolve(_ name: String) -> Selector {
        let selector = NSSelectorFromString(name)
        return window.responds(to: selector) ? selector : NSSelectorFromString("_\(name)")
    }
    let getter = resolve(key)
    let setter = resolve("set\(key.prefix(1).uppercased())\(key.dropFirst()):")
    func encoding(_ type: UnsafeMutablePointer<CChar>?) -> String {
        guard let type else { return "" }
        defer { free(type) }
        return String(cString: type)
    }
    guard window.responds(to: getter), window.responds(to: setter),
          let getMethod = class_getInstanceMethod(type(of: window), getter),
          let setMethod = class_getInstanceMethod(type(of: window), setter),
          method_getNumberOfArguments(getMethod) == 2,
          method_getNumberOfArguments(setMethod) == 3,
          ["B", "c"].contains(encoding(method_copyReturnType(getMethod))),
          encoding(method_copyReturnType(setMethod)) == "v",
          ["B", "c"].contains(encoding(method_copyArgumentType(setMethod, 2))) else {
        throw NSError(domain: "AppearanceTrial", code: 1,
                      userInfo: [NSLocalizedDescriptionKey: "当前系统不支持 BOOL 属性 \(key)"])
    }
    if let value {
        let implementation = method_getImplementation(setMethod)
        if encoding(method_copyArgumentType(setMethod, 2)) == "B" {
            typealias Setter = @convention(c) (AnyObject, Selector, Bool) -> Void
            unsafeBitCast(implementation, to: Setter.self)(window, setter, value)
        } else {
            typealias Setter = @convention(c) (AnyObject, Selector, Int8) -> Void
            unsafeBitCast(implementation, to: Setter.self)(window, setter, value ? 1 : 0)
        }
    }
    let result: Bool
    if encoding(method_copyReturnType(getMethod)) == "B" {
        typealias Getter = @convention(c) (AnyObject, Selector) -> Bool
        result = unsafeBitCast(method_getImplementation(getMethod), to: Getter.self)(window, getter)
    } else {
        typealias Getter = @convention(c) (AnyObject, Selector) -> Int8
        result = unsafeBitCast(method_getImplementation(getMethod), to: Getter.self)(window, getter) != 0
    }
    if let value, result != value {
        throw NSError(domain: "AppearanceTrial", code: 3,
                      userInfo: [NSLocalizedDescriptionKey: "\(key) 设置后未生效"])
    }
    return result
}

@available(macOS 26.0, *)
@MainActor
final class AppearanceControl: NSObject, NSApplicationDelegate, NSWindowDelegate {
    private var window: NSWindow!
    private let names = ["原生 Liquid Glass", "原生传统毛玻璃", "不透明背景"]
    private let started = Date()
    private var materialMode = 0
    private var trial = WindowTrial.baseline
    private var trialFailure: String?
    private var generation = 0

    func applicationDidFinishLaunching(_ notification: Notification) {
        let menu = NSMenu()
        let appItem = NSMenuItem()
        let appMenu = NSMenu()
        let quit = NSMenuItem(title: "退出对照程序", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        appMenu.addItem(quit)
        appItem.submenu = appMenu
        menu.addItem(appItem)
        let modesItem = NSMenuItem()
        let modesMenu = NSMenu(title: "材质")
        for (index, name) in names.enumerated() {
            let item = NSMenuItem(title: name, action: #selector(selectMenuMode(_:)), keyEquivalent: "\(index + 1)")
            item.target = self
            item.tag = index
            modesMenu.addItem(item)
        }
        modesItem.submenu = modesMenu
        menu.addItem(modesItem)
        let trialsItem = NSMenuItem()
        let trialsMenu = NSMenu(title: "实验")
        for (index, trial) in WindowTrial.allCases.enumerated() {
            let item = NSMenuItem(title: trial.title, action: #selector(selectTrialMenu(_:)), keyEquivalent: "\(index + 1)")
            item.keyEquivalentModifierMask = [.command, .option]
            item.target = self
            item.tag = index
            trialsMenu.addItem(item)
        }
        trialsItem.submenu = trialsMenu
        menu.addItem(trialsItem)
        NSApp.mainMenu = menu

        if CommandLine.arguments.contains("--probe-window-api") {
            var current: AnyClass? = NSWindow.self
            while let cls = current {
                var count: UInt32 = 0
                if let methods = class_copyMethodList(cls, &count) {
                    defer { free(methods) }
                    for index in 0..<Int(count) {
                        let method = methods[index]
                        let name = NSStringFromSelector(method_getName(method))
                        if name.localizedCaseInsensitiveContains("flatten") || name.contains("HostLayers") {
                            print("\(NSStringFromClass(cls)).\(name): \(method_getTypeEncoding(method).map(String.init(cString:)) ?? "unknown")")
                        }
                    }
                }
                current = class_getSuperclass(cls)
            }
            exit(0)
        }
        if let argument = CommandLine.arguments.first(where: { $0.hasPrefix("--variant=") }) {
            guard let selected = WindowTrial(rawValue: String(argument.dropFirst("--variant=".count))) else {
                fputs("未知实验参数：\(argument)\n", stderr)
                exit(2)
            }
            trial = selected
        }
        if CommandLine.arguments.contains("--check-configurations") {
            checkConfigurations()
            return
        }
        rebuildWindow()
        NSApp.activate()
    }

    private func rebuildWindow(show: Bool = true) {
        let previous = window
        let newWindow = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1100, height: 740),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered, defer: false
        )
        window = newWindow
        generation += 1
        trialFailure = nil
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.appearance = NSAppearance(named: .darkAqua)
        window.minSize = NSSize(width: 700, height: 500)
        window.hasShadow = trial != .noShadow
        // 应在首次显示之前设置，焦点回调不得重建材质或图层。
        if trial.disablesFlattening {
            do { _ = try checkedBoolProperty(window, "shouldAutoFlattenLayerTree", set: false) }
            catch { trialFailure = error.localizedDescription }
        }
        buildContent()
        if trial.rehostsLayers && trialFailure == nil {
            do {
                _ = try checkedBoolProperty(window, "canHostLayersInWindowServer", set: false)
                _ = try checkedBoolProperty(window, "canHostLayersInWindowServer", set: true)
            } catch {
                trialFailure = error.localizedDescription
                // 如果关闭成功而重新启用失败，仅尝试恢复，不把半配置窗口当成有效实验。
                _ = try? checkedBoolProperty(window, "canHostLayersInWindowServer", set: true)
            }
            if trialFailure != nil { buildContent() }
        }
        window.center()
        if show { window.makeKeyAndOrderFront(nil) }
        previous?.delegate = nil
        previous?.close()
        logState("READY")
    }

    @objc private func selectMenuMode(_ sender: NSMenuItem) {
        materialMode = sender.tag
        rebuildWindow()
    }

    @objc private func selectControlMode(_ sender: NSSegmentedControl) {
        materialMode = sender.selectedSegment
        rebuildWindow()
    }

    @objc private func selectTrialMenu(_ sender: NSMenuItem) {
        trial = WindowTrial.allCases[sender.tag]
        rebuildWindow()
    }

    @objc private func selectTrialControl(_ sender: NSPopUpButton) {
        trial = WindowTrial.allCases[sender.indexOfSelectedItem]
        rebuildWindow()
    }

    private func buildContent() {
        let workspace = NSWorkspace.shared
        let solidRequired = workspace.accessibilityDisplayShouldReduceTransparency
            || workspace.accessibilityDisplayShouldIncreaseContrast
        let mode = solidRequired ? 2 : materialMode
        let rect = NSRect(origin: .zero, size: window.frame.size)
        let content = NSView(frame: rect)
        content.autoresizingMask = [.width, .height]

        let root: NSView
        if mode == 0 {
            let glass = NSGlassEffectView(frame: rect)
            glass.style = .regular
            glass.cornerRadius = 16
            // 使用 AppKit 保证的 contentView 层级，不依赖第三方材质库。
            glass.contentView = content
            root = glass
        } else if mode == 1 {
            let blur = NSVisualEffectView(frame: rect)
            blur.material = .sidebar
            blur.blendingMode = .behindWindow
            blur.state = .active
            blur.addSubview(content)
            root = blur
        } else {
            root = content
        }
        window.isOpaque = mode == 2
        window.backgroundColor = mode == 2
            ? NSColor(calibratedRed: 0.13, green: 0.17, blue: 0.22, alpha: 1)
            : NSColor.white.withAlphaComponent(trial.backgroundAlpha)
        window.contentView = root
        window.title = "\(trial.title) — \(names[mode])"

        let title = NSTextField(labelWithString: "\(trial.title)")
        title.font = .boldSystemFont(ofSize: 25)
        let current = NSTextField(labelWithString: "当前：\(names[mode])")
        current.font = .systemFont(ofSize: 18)
        let controls = NSSegmentedControl(
            labels: ["⌘1 液态玻璃", "⌘2 传统毛玻璃", "⌘3 不透明"],
            trackingMode: .selectOne, target: self, action: #selector(selectControlMode(_:))
        )
        controls.selectedSegment = mode
        controls.isEnabled = !solidRequired
        let trialControl = NSPopUpButton()
        trialControl.addItems(withTitles: WindowTrial.allCases.map(\.title))
        trialControl.selectItem(at: WindowTrial.allCases.firstIndex(of: trial)!)
        trialControl.target = self
        trialControl.action = #selector(selectTrialControl(_:))
        let explanation = NSTextField(labelWithString: "无 Tauri / WebView；切到后台后，从 Dock 或台前调度点回来。")
        explanation.font = .systemFont(ofSize: 13)
        let status = NSTextField(labelWithString: trialFailure.map { "实验未启用：\($0)" }
            ?? "每次选择均新建窗口；焦点变化时不修改外观。")
        status.textColor = trialFailure == nil ? .secondaryLabelColor : .systemRed
        status.font = .systemFont(ofSize: 12)
        let stack = NSStackView(views: [title, current, controls, trialControl, explanation, status])
        stack.orientation = .vertical
        stack.alignment = .centerX
        stack.spacing = 22
        stack.translatesAutoresizingMaskIntoConstraints = false
        let card = NSBox()
        card.boxType = .custom
        card.borderWidth = 0
        card.fillColor = NSColor(calibratedWhite: 0.16, alpha: 1)
        card.cornerRadius = 20
        card.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(card)
        card.addSubview(stack)
        NSLayoutConstraint.activate([
            card.centerXAnchor.constraint(equalTo: content.centerXAnchor),
            card.centerYAnchor.constraint(equalTo: content.centerYAnchor),
            stack.leadingAnchor.constraint(equalTo: card.leadingAnchor, constant: 36),
            stack.trailingAnchor.constraint(equalTo: card.trailingAnchor, constant: -36),
            stack.topAnchor.constraint(equalTo: card.topAnchor, constant: 36),
            stack.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: -36),
        ])
        root.layoutSubtreeIfNeeded()
        assert(window.isOpaque == (mode == 2))
        assert(window.contentView === root)
    }

    private func logState(_ event: String) {
        let flatten = (try? checkedBoolProperty(window, "shouldAutoFlattenLayerTree"))
            .map(String.init) ?? "unsupported"
        let hosting = (try? checkedBoolProperty(window, "canHostLayersInWindowServer"))
            .map(String.init) ?? "unsupported"
        print("[native +\(Date().timeIntervalSince(started))] \(event) generation=\(generation) variant=\(trial.rawValue) material=\(materialMode + 1) opaque=\(window.isOpaque) alpha=\(window.backgroundColor?.alphaComponent ?? -1) shadow=\(window.hasShadow) flatten=\(flatten) hosting=\(hosting) error=\(trialFailure ?? "none")")
        fflush(stdout)
    }

    func windowDidBecomeKey(_ notification: Notification) {
        guard notification.object as? NSWindow === window else { return }
        logState("focused=true")
        // 只读核对恢复后一秒的配置，观察 AppKit 是否又改回自动扁平化；不触发重绘。
        let expectedGeneration = generation
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
            guard let self, self.generation == expectedGeneration, self.window.isKeyWindow else { return }
            self.logState("focused+1.2s")
        }
    }

    func windowDidResignKey(_ notification: Notification) {
        guard notification.object as? NSWindow === window else { return }
        logState("focused=false")
    }

    private func checkConfigurations() {
        var failed = false
        for variant in WindowTrial.allCases {
            trial = variant
            rebuildWindow(show: false)
            if trialFailure != nil { failed = true; continue }
            guard !window.isOpaque else {
                fputs("SKIP: 系统辅助功能要求不透明，无法验证玻璃实验。\n", stderr)
                exit(2)
            }
            precondition(abs((window.backgroundColor?.alphaComponent ?? -1) - trial.backgroundAlpha) < 0.000001)
            precondition(window.hasShadow == (trial != .noShadow))
            let glass = window.contentView as! NSGlassEffectView
            precondition(glass.contentView != nil)
            if trial.disablesFlattening {
                precondition((try? checkedBoolProperty(window, "shouldAutoFlattenLayerTree")) == false)
            }
            if trial.rehostsLayers {
                precondition((try? checkedBoolProperty(window, "canHostLayersInWindowServer")) == true)
            }
        }
        print(failed ? "FAIL: 至少一项私有接口实验不可用" : "PASS: 六种原生配置与玻璃内容层级；未验证过渡帧")
        fflush(stdout)
        exit(failed ? 1 : 0)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}

if #available(macOS 26.0, *) {
    // 可执行程序的顶层入口位于主线程；AppKit 事件循环始终留在该线程。
    MainActor.assumeIsolated {
        let app = NSApplication.shared
        let delegate = AppearanceControl()
        app.setActivationPolicy(.regular)
        app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }
} else {
    fputs("此对照程序需要 macOS 26 或更高版本。\n", stderr)
    exit(1)
}
