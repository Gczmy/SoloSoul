import UIKit
import Flutter
import LocalAuthentication
import Security

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
}
