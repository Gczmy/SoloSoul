import Foundation
import CoreText
let text = "欢迎来到 SoloSoul" as CFString
let system = CTFontCreateUIFontForLanguage(.system, 17, "zh-Hans" as CFString)!
let fallback = CTFontCreateForString(system, text, CFRange(location: 0, length: 4))
let chars: [UniChar] = Array("欢迎来到".utf16)
var glyphs = [CGGlyph](repeating: 0, count: chars.count)
let hasGlyphs = CTFontGetGlyphsForCharacters(fallback, chars, &glyphs, chars.count)
let record: [String: Any] = ["systemFont": CTFontCopyPostScriptName(system) as String,
 "fallbackFont": CTFontCopyPostScriptName(fallback) as String, "hasCJKGlyphs": hasGlyphs,
 "glyphs": glyphs.map(Int.init), "availableFontFamilies": (CTFontManagerCopyAvailableFontFamilyNames() as NSArray).count]
print(String(data: try! JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]), encoding: .utf8)!)
