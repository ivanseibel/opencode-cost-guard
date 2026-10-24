import CoreGraphics
import Foundation

let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
guard let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
    exit(1)
}

for window in windows {
    let owner = window[kCGWindowOwnerName as String] as? String ?? ""
    let name = window[kCGWindowName as String] as? String ?? ""
    let haystack = "\(owner) \(name)".lowercased()
    guard haystack.contains("cost guard") || haystack.contains("cost-guard") else { continue }
    let number = window[kCGWindowNumber as String] ?? "?"
    let bounds = window[kCGWindowBounds as String] ?? ""
    print("id=\(number) owner=\(owner) name=\(name) bounds=\(bounds)")
}
