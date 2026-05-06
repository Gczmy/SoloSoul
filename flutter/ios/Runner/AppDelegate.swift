import UIKit
import Flutter
import LocalAuthentication
import Security
import Vision

@main
class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    // Setup native channels for Keychain and biometrics
    setupNativeChannels()

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  private func setupNativeChannels() {
    // Get the Flutter engine
    guard let controller = window?.rootViewController as? FlutterViewController else {
      return
    }

    let flutterEngine = controller.engine

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

    // Setup native channel for vault operations
    // Note: DistributedNotificationCenter is macOS-only and not available on iOS.
    // On iOS, vault lock operations are handled directly via method channel within the same process.
    let nativeChannel = FlutterMethodChannel(
      name: "com.solosoul/native",
      binaryMessenger: flutterEngine.binaryMessenger
    )

    nativeChannel.setMethodCallHandler { [weak self] (call: FlutterMethodCall, result: @escaping FlutterResult) in
      switch call.method {
      case "lockVault":
        // iOS is single-process with one Flutter engine - no cross-process notification needed.
        // The vault lock is handled directly by the Rust service via Flutter's native_crypto_service.
        // Return success to indicate the lock request was received.
        result(nil)
      default:
        result(FlutterMethodNotImplemented)
      }
    }

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

  // MARK: - Apple Vision OCR

  private func recognizeTextWithVision(imageData: Data, result: @escaping FlutterResult) {
    guard let image = UIImage(data: imageData),
          let cgImage = image.cgImage else {
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
