import Cocoa
import FlutterMacOS
import LocalAuthentication
import Security
import Vision

@main
class AppDelegate: FlutterAppDelegate {
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return true
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }

  override func applicationDidFinishLaunching(_ notification: Notification) {
    // Setup native channel for Keychain and biometrics
    setupNativeChannels()

    // Setup power event monitoring (sleep/wake)
    setupPowerEvents()

    // Setup menu bar
    setupMenuBar()

    // Setup system tray icon
    setupTrayIcon()
  }

  private func setupNativeChannels() {
    // Get the Flutter engine via window
    guard let flutterWindow = mainFlutterWindow,
          let flutterViewController = flutterWindow.contentViewController as? FlutterViewController else {
      return
    }

    let flutterEngine = flutterViewController.engine

    let keychainChannel = FlutterMethodChannel(
      name: "com.solosoul/keychain",
      binaryMessenger: flutterEngine.binaryMessenger
    )

    keychainChannel.setMethodCallHandler { [weak self] (call: FlutterMethodCall, result: @escaping FlutterResult) in
      switch call.method {
      case "authenticateWithBiometrics":
        self?.authenticateWithBiometrics(result: result)
      case "saveToKeychain":
        if let args = call.arguments as? [String: Any],
           let key = args["key"] as? String,
           let value = args["value"] as? String {
          self?.saveToKeychain(key: key, value: value, result: result)
        } else {
          result(FlutterError(code: "INVALID_ARGS", message: "Missing key or value", details: nil))
        }
      case "readFromKeychain":
        if let args = call.arguments as? [String: Any],
           let key = args["key"] as? String {
          self?.readFromKeychain(key: key, result: result)
        } else {
          result(FlutterError(code: "INVALID_ARGS", message: "Missing key", details: nil))
        }
      case "deleteFromKeychain":
        if let args = call.arguments as? [String: Any],
           let key = args["key"] as? String {
          self?.deleteFromKeychain(key: key, result: result)
        } else {
          result(FlutterError(code: "INVALID_ARGS", message: "Missing key", details: nil))
        }
      default:
        result(FlutterMethodNotImplemented)
      }
    }

    // Setup lock vault notification observer
    DistributedNotificationCenter.default().addObserver(
      self,
      selector: #selector(handleLockVault),
      name: NSNotification.Name("com.solosoul.lockVault"),
      object: nil
    )

    // Setup OCR Vision channel
    let ocrVisionChannel = FlutterMethodChannel(
      name: "com.solosoul/ocr.vision",
      binaryMessenger: flutterEngine.binaryMessenger
    )

    ocrVisionChannel.setMethodCallHandler { [weak self] (call: FlutterMethodCall, result: @escaping FlutterResult) in
      switch call.method {
      case "recognizeText":
        if let args = call.arguments as? [String: Any],
           let imageData = args["imageData"] as? FlutterStandardTypedData {
          self?.recognizeTextWithVision(imageData: imageData.data, result: result)
        } else {
          result(FlutterError(code: "INVALID_ARGS", message: "Missing imageData", details: nil))
        }
      default:
        result(FlutterMethodNotImplemented)
      }
    }

    // Setup QuickLook channel for PPTX/PPT preview
    QuickLookHandler.shared.register(with: flutterEngine)
  }

  @objc private func handleLockVault() {
    sendToFlutter(method: "lockVault")
  }

  // MARK: - System Tray

  private func setupTrayIcon() {
    guard let icon = NSImage(named: "TrayIcon") else {
      return
    }
    icon.isTemplate = true  // Auto-adapts to dark/light menu bar
    icon.size = NSSize(width: 16, height: 16)

    let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    if let button = statusItem.button {
      button.image = icon
      button.toolTip = "SoloSoul"
    }

    let menu = NSMenu()
    menu.addItem(withTitle: "Show SoloSoul", action: #selector(showApp), keyEquivalent: "")
    menu.addItem(NSMenuItem.separator())
    menu.addItem(withTitle: "Lock Vault", action: #selector(lockVaultFromMenu), keyEquivalent: "l")
    menu.addItem(NSMenuItem.separator())
    menu.addItem(withTitle: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
    statusItem.menu = menu
  }

  @objc private func showApp() {
    NSApp.activate(ignoringOtherApps: true)
    mainFlutterWindow?.makeKeyAndOrderFront(nil)
  }

  // MARK: - Power Events (Sleep / Wake)

  private func setupPowerEvents() {
    NSWorkspace.shared.notificationCenter.addObserver(
      self,
      selector: #selector(handleWillSleep),
      name: NSWorkspace.willSleepNotification,
      object: nil
    )
    NSWorkspace.shared.notificationCenter.addObserver(
      self,
      selector: #selector(handleDidWake),
      name: NSWorkspace.didWakeNotification,
      object: nil
    )
  }

  @objc private func handleWillSleep() {
    // Lock vault before system sleeps to clear sensitive keys from memory
    sendToFlutter(method: "onSystemWillSleep")
  }

  @objc private func handleDidWake() {
    // Notify Dart to re-validate session after wake
    sendToFlutter(method: "onSystemDidWake")
  }

  /// Helper to send method calls to Dart via the native channel
  private func sendToFlutter(method: String, arguments: Any? = nil) {
    guard let flutterWindow = mainFlutterWindow,
          let flutterViewController = flutterWindow.contentViewController as? FlutterViewController else {
      return
    }
    let flutterEngine = flutterViewController.engine
    let channel = FlutterMethodChannel(
      name: "com.solosoul/native",
      binaryMessenger: flutterEngine.binaryMessenger
    )
    channel.invokeMethod(method, arguments: arguments)
  }

  // MARK: - TouchID / Biometrics

  private func authenticateWithBiometrics(result: @escaping FlutterResult) {
    let context = LAContext()
    var error: NSError?

    if context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) {
      context.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics,
                            localizedReason: "Unlock SoloSoul Vault") { success, authError in
        DispatchQueue.main.async {
          if success {
            result(["success": true, "biometryType": self.biometryType()])
          } else {
            result(["success": false, "error": authError?.localizedDescription ?? "Authentication failed"])
          }
        }
      }
    } else {
      result(["success": false, "error": error?.localizedDescription ?? "Biometrics not available"])
    }
  }

  private func biometryType() -> String {
    let context = LAContext()
    var error: NSError?

    guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) else {
      return "none"
    }

    switch context.biometryType {
    case .faceID:
      return "faceID"
    case .touchID:
      return "touchID"
    case .opticID:
      return "opticID"
    case .none:
      return "none"
    @unknown default:
      return "unknown"
    }
  }

  // MARK: - Keychain

  private func saveToKeychain(key: String, value: String, result: @escaping FlutterResult) {
    guard let data = value.data(using: .utf8) else {
      result(["success": false, "error": "Invalid string data"])
      return
    }

    let deleteQuery: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrAccount as String: key,
      kSecAttrService as String: "com.solosoul"
    ]
    SecItemDelete(deleteQuery as CFDictionary)

    let addQuery: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrAccount as String: key,
      kSecAttrService as String: "com.solosoul",
      kSecValueData as String: data,
      kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
    ]

    let status = SecItemAdd(addQuery as CFDictionary, nil)
    if status == errSecSuccess {
      result(["success": true])
    } else {
      result(["success": false, "error": "Keychain save failed: \(status)"])
    }
  }

  private func readFromKeychain(key: String, result: @escaping FlutterResult) {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrAccount as String: key,
      kSecAttrService as String: "com.solosoul",
      kSecReturnData as String: true,
      kSecMatchLimit as String: kSecMatchLimitOne
    ]

    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)

    if status == errSecSuccess, let data = item as? Data, let value = String(data: data, encoding: .utf8) {
      result(["success": true, "value": value])
    } else if status == errSecItemNotFound {
      result(["success": false, "error": "Item not found"])
    } else {
      result(["success": false, "error": "Keychain read failed: \(status)"])
    }
  }

  private func deleteFromKeychain(key: String, result: @escaping FlutterResult) {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrAccount as String: key,
      kSecAttrService as String: "com.solosoul"
    ]

    let status = SecItemDelete(query as CFDictionary)
    if status == errSecSuccess || status == errSecItemNotFound {
      result(["success": true])
    } else {
      result(["success": false, "error": "Keychain delete failed: \(status)"])
    }
  }

  // MARK: - Menu Bar

  private func setupMenuBar() {
    let mainMenu = NSMenu()

    // App menu
    let appMenuItem = NSMenuItem()
    let appMenu = NSMenu()
    appMenu.addItem(withTitle: "About SoloSoul", action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)), keyEquivalent: "")
    appMenu.addItem(NSMenuItem.separator())
    appMenu.addItem(withTitle: "Preferences...", action: nil, keyEquivalent: ",")
    appMenu.addItem(NSMenuItem.separator())
    appMenu.addItem(withTitle: "Hide SoloSoul", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
    appMenu.addItem(withTitle: "Hide Others", action: #selector(NSApplication.hideOtherApplications(_:)), keyEquivalent: "h")
    appMenu.addItem(withTitle: "Show All", action: #selector(NSApplication.unhideAllApplications(_:)), keyEquivalent: "")
    appMenu.addItem(NSMenuItem.separator())
    appMenu.addItem(withTitle: "Quit SoloSoul", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
    appMenuItem.submenu = appMenu
    mainMenu.addItem(appMenuItem)

    // File menu
    let fileMenuItem = NSMenuItem()
    let fileMenu = NSMenu(title: "File")
    fileMenu.addItem(withTitle: "Lock Vault", action: #selector(lockVaultFromMenu), keyEquivalent: "l")
    fileMenu.addItem(NSMenuItem.separator())
    fileMenu.addItem(withTitle: "Close Window", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
    fileMenuItem.submenu = fileMenu
    mainMenu.addItem(fileMenuItem)

    // Edit menu
    let editMenuItem = NSMenuItem()
    let editMenu = NSMenu(title: "Edit")
    editMenu.addItem(withTitle: "Undo", action: Selector(("undo:")), keyEquivalent: "z")
    editMenu.addItem(withTitle: "Redo", action: Selector(("redo:")), keyEquivalent: "Z")
    editMenu.addItem(NSMenuItem.separator())
    editMenu.addItem(withTitle: "Cut", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
    editMenu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
    editMenu.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
    editMenu.addItem(withTitle: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")
    editMenuItem.submenu = editMenu
    mainMenu.addItem(editMenuItem)

    // View menu
    let viewMenuItem = NSMenuItem()
    let viewMenu = NSMenu(title: "View")
    viewMenu.addItem(withTitle: "Enter Full Screen", action: #selector(NSWindow.toggleFullScreen(_:)), keyEquivalent: "f")
    viewMenuItem.submenu = viewMenu
    mainMenu.addItem(viewMenuItem)

    // Window menu
    let windowMenuItem = NSMenuItem()
    let windowMenu = NSMenu(title: "Window")
    windowMenu.addItem(withTitle: "Minimize", action: #selector(NSWindow.performMiniaturize(_:)), keyEquivalent: "m")
    windowMenu.addItem(withTitle: "Zoom", action: #selector(NSWindow.performZoom(_:)), keyEquivalent: "")
    windowMenu.addItem(NSMenuItem.separator())
    windowMenu.addItem(withTitle: "Bring All to Front", action: #selector(NSApplication.arrangeInFront(_:)), keyEquivalent: "")
    windowMenuItem.submenu = windowMenu
    mainMenu.addItem(windowMenuItem)

    // Help menu
    let helpMenuItem = NSMenuItem()
    let helpMenu = NSMenu(title: "Help")
    helpMenu.addItem(withTitle: "SoloSoul Help", action: #selector(NSApplication.showHelp(_:)), keyEquivalent: "?")
    helpMenuItem.submenu = helpMenu
    mainMenu.addItem(helpMenuItem)

    NSApplication.shared.mainMenu = mainMenu
  }

  @objc private func lockVaultFromMenu() {
    handleLockVault()
  }

  // MARK: - Apple Vision OCR

  private func recognizeTextWithVision(imageData: Data, result: @escaping FlutterResult) {
    guard let image = NSImage(data: imageData),
          let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
      result(FlutterError(code: "INVALID_IMAGE", message: "Could not decode image", details: nil))
      return
    }

    let request = VNRecognizeTextRequest { [weak self] (request, error) in
      guard let self = self else { return }

      if let error = error {
        DispatchQueue.main.async {
          result(FlutterError(code: "VISION_ERROR", message: error.localizedDescription, details: nil))
        }
        return
      }

      guard let observations = request.results as? [VNRecognizedTextObservation] else {
        DispatchQueue.main.async {
          result(["rawText": "", "blocks": [], "confidence": 0.0])
        }
        return
      }

      let blocks = self.processVisionObservations(observations)
      let rawText = blocks.map { $0["text"] as? String ?? "" }.joined(separator: "\n")
      let avgConfidence = blocks.isEmpty ? 0.0 : (blocks.reduce(0.0) { $0 + ($1["confidence"] as? Double ?? 0.0) }) / Double(blocks.count)

      DispatchQueue.main.async {
        result([
          "rawText": rawText,
          "blocks": blocks,
          "confidence": avgConfidence
        ])
      }
    }

    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = true

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

    DispatchQueue.global(qos: .userInitiated).async {
      do {
        try handler.perform([request])
      } catch {
        DispatchQueue.main.async {
          result(FlutterError(code: "VISION_ERROR", message: error.localizedDescription, details: nil))
        }
      }
    }
  }

  private func processVisionObservations(_ observations: [VNRecognizedTextObservation]) -> [[String: Any]] {
    // Vision boundingBox uses CoreGraphics coords: origin bottom-left, y-up
    // Convert to Flutter coords: origin top-left, y-down
    var blocks: [[String: Any]] = []

    for observation in observations {
      guard let candidate = observation.topCandidates(1).first else { continue }

      let text = candidate.string
      let confidence = Double(observation.confidence)
      let bbox = observation.boundingBox

      // Convert CG coords to Flutter relative coords
      let x = bbox.origin.x
      let y = 1.0 - bbox.origin.y - bbox.height
      let width = bbox.width
      let height = bbox.height

      blocks.append([
        "text": text,
        "confidence": confidence,
        "bbox": [
          "x": x,
          "y": y,
          "width": width,
          "height": height
        ]
      ])
    }

    // Sort by reading order: top-to-bottom, left-to-right
    blocks.sort {
      let aBbox = $0["bbox"] as? [String: Double] ?? [:]
      let bBbox = $1["bbox"] as? [String: Double] ?? [:]
      let aY = aBbox["y"] ?? 0.0
      let bY = bBbox["y"] ?? 0.0
      let aX = aBbox["x"] ?? 0.0
      let bX = bBbox["x"] ?? 0.0

      // Group by rows (y within half height difference)
      let aH = aBbox["height"] ?? 0.0
      let bH = bBbox["height"] ?? 0.0
      let minH = min(aH, bH)

      if abs(aY - bY) < minH * 0.5 {
        return aX < bX
      } else {
        return aY < bY
      }
    }

    return blocks
  }
}
