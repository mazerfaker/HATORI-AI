import UIKit
import SwiftUI

enum CrimsonState: String {
    case idle, thinking, responding, completed, cancelled, failed
    var label: String {
        switch self {
        case .idle: return "Очікування"
        case .thinking: return "Обробка запиту"
        case .responding: return "Генерація відповіді"
        case .completed: return "Відповідь готова"
        case .cancelled: return "Зупинено"
        case .failed: return "Помилка"
        }
    }
    var animated: Bool { self == .thinking || self == .responding }
}

@MainActor enum CrimsonFeedback {
    static func send(enabled: Bool) {
        guard enabled else { return }
        UIImpactFeedbackGenerator(style: .light).impactOccurred()
    }
    static func finish(_ state: CrimsonState, enabled: Bool) {
        guard enabled else { return }
        switch state {
        case .completed: UINotificationFeedbackGenerator().notificationOccurred(.success)
        case .failed: UINotificationFeedbackGenerator().notificationOccurred(.error)
        case .cancelled: UIImpactFeedbackGenerator(style: .soft).impactOccurred()
        default: break
        }
    }
}
