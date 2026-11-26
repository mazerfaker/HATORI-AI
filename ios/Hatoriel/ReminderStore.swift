import Foundation
import SwiftUI
import UserNotifications

struct IOSReminder: Codable, Identifiable {
    let id: UUID
    let text: String
    let due: Date
}
struct ReminderPreferences: Codable {
    var enabled = false
    var quiet = false
    var sound = true
    var showText = false
    var checkIn = false
    var checkInMinutes = 120
    var reminders: [IOSReminder] = []
}

@MainActor final class ReminderStore: ObservableObject {
    static let prefix = "hatoriel.reminder."
    static let checkInID = "hatoriel.reminder.checkin"
    @Published private(set) var data = ReminderPreferences()
    @Published private(set) var authorization: UNAuthorizationStatus = .notDetermined
    @Published private(set) var pending: Set<String> = []
    @Published private(set) var working = false
    @Published private(set) var storageError: String?
    @Published private(set) var status = ""
    private let center = UNUserNotificationCenter.current()

    init() { load() }
    private static func file() throws -> URL {
        let fm = FileManager.default
        var dir = try fm.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true).appendingPathComponent("Reminders", isDirectory: true)
        try fm.createDirectory(at: dir, withIntermediateDirectories: true)
        var values = URLResourceValues(); values.isExcludedFromBackup = true
        try dir.setResourceValues(values)
        return dir.appendingPathComponent("reminders.json")
    }
    private func load() {
        do {
            let url = try Self.file()
            if FileManager.default.fileExists(atPath: url.path) {
                let bytes = try Data(contentsOf: url)
                guard bytes.count <= 262144 else { throw CoreFailure.rejected("Сховище нагадувань перевищує ліміт.") }
                let saved = try JSONDecoder().decode(ReminderPreferences.self, from: bytes)
                guard saved.reminders.count <= 40, Set(saved.reminders.map(\.id)).count == saved.reminders.count,
                      saved.reminders.allSatisfy({ !$0.text.isEmpty && $0.text.unicodeScalars.count <= 500 }),
                      (60...1440).contains(saved.checkInMinutes) else { throw CoreFailure.rejected("Некоректний формат нагадувань.") }
                data = saved
            } else { data = ReminderPreferences() }
            storageError = nil
        } catch { storageError = error.localizedDescription }
    }
    private func persist(_ proposed: ReminderPreferences) throws {
        let bytes = try JSONEncoder().encode(proposed)
        guard bytes.count <= 262144 else { throw CoreFailure.rejected("Сховище перевищує ліміт.") }
        try bytes.write(to: Self.file(), options: [.atomic, .completeFileProtection])
        data = proposed
    }
    var allowed: Bool { [.authorized, .provisional, .ephemeral].contains(authorization) }
    var authorizationLabel: String {
        switch authorization {
        case .notDetermined: return "Ще не запитано"
        case .denied: return "Заборонено в iOS"
        case .authorized: return "Дозволено"
        case .provisional: return "Тиха доставка"
        case .ephemeral: return "Тимчасовий дозвіл"
        @unknown default: return "Невідомо"
        }
    }
    func requestPermission() async {
        guard !working else { return }
        working = true; defer { working = false }
        do {
            _ = try await center.requestAuthorization(options: [.alert, .sound])
            await reconcile()
        } catch { status = error.localizedDescription }
    }
    func refresh() async {
        guard !working else { return }
        working = true; defer { working = false }
        if storageError != nil { load() }
        await reconcile()
    }
    func saveSettings(enabled: Bool, quiet: Bool, sound: Bool, showText: Bool, checkIn: Bool, minutes: Int) async {
        await change(force: true) {
            $0.enabled = enabled; $0.quiet = quiet; $0.sound = sound; $0.showText = showText
            $0.checkIn = checkIn; $0.checkInMinutes = min(1440,max(60,minutes))
        }
    }
    func add(text: String, minutes: Int) async {
        await change {
            let value = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !value.isEmpty, value.unicodeScalars.count <= 500, (1...10080).contains(minutes) else {
                throw CoreFailure.rejected("Вкажи 1–500 символів і час від 1 до 10080 хвилин.")
            }
            guard $0.reminders.count < 40 else { throw CoreFailure.rejected("Ліміт 40 записів. Видали завершені нагадування.") }
            $0.reminders.append(IOSReminder(id: UUID(), text: value, due: Date().addingTimeInterval(Double(minutes)*60)))
        }
    }
    func remove(_ id: UUID) async { await change { $0.reminders.removeAll { $0.id == id } } }
    private func change(force: Bool = false, _ edit: (inout ReminderPreferences) throws -> Void) async {
        guard !working, storageError == nil else { return }
        working = true; defer { working = false }
        do {
            var proposed = data
            try edit(&proposed)
            try persist(proposed)
            await reconcile(force: force)
        } catch { status = error.localizedDescription }
    }
    private func content(_ text: String, checkIn: Bool = false) -> UNMutableNotificationContent {
        let c = UNMutableNotificationContent()
        c.title = "Hatoriel"
        c.body = checkIn ? "Час перевірити свої цілі та завдання." : (data.showText ? text : "У тебе є заплановане нагадування.")
        if data.sound { c.sound = .default }
        c.interruptionLevel = .active
        return c
    }
    private func reconcile(force: Bool = false) async {
        authorization = (await center.notificationSettings()).authorizationStatus
        let requests = await center.pendingNotificationRequests()
        let ours = requests.filter { $0.identifier.hasPrefix(Self.prefix) }
        if storageError != nil {
            status = "Сховище недоступне. Наявні системні нагадування не змінено."
            pending = Set(ours.map(\.identifier)); return
        }
        guard allowed && data.enabled && !data.quiet else {
            center.removePendingNotificationRequests(withIdentifiers: ours.map(\.identifier))
            pending = []
            status = data.quiet ? "Режим тиші активний" : "Сповіщення вимкнені або не дозволені"
            return
        }
        let future = data.reminders.filter { $0.due > Date() }
        var desired = Set(future.map { Self.prefix + $0.id.uuidString })
        if data.checkIn { desired.insert(Self.checkInID) }
        let stale = ours.filter { !desired.contains($0.identifier) }
        center.removePendingNotificationRequests(withIdentifiers: stale.map(\.identifier))
        let existing = force ? Set<String>() : Set(ours.map(\.identifier)).subtracting(stale.map(\.identifier))
        var failures = 0
        for row in future {
            let id = Self.prefix + row.id.uuidString
            guard !existing.contains(id) else { continue }
            let seconds = row.due.timeIntervalSinceNow
            guard seconds > 0 else { continue }
            let trigger = UNTimeIntervalNotificationTrigger(timeInterval: max(1, seconds), repeats: false)
            do { try await center.add(UNNotificationRequest(identifier: id, content: content(row.text), trigger: trigger)) }
            catch { failures += 1; if force { center.removePendingNotificationRequests(withIdentifiers: [id]) } }
        }
        if data.checkIn && !existing.contains(Self.checkInID) {
            let trigger = UNTimeIntervalNotificationTrigger(timeInterval: Double(data.checkInMinutes)*60, repeats: true)
            do { try await center.add(UNNotificationRequest(identifier: Self.checkInID, content: content("", checkIn: true), trigger: trigger)) }
            catch { failures += 1; if force { center.removePendingNotificationRequests(withIdentifiers: [Self.checkInID]) } }
        }
        pending = Set((await center.pendingNotificationRequests()).filter { $0.identifier.hasPrefix(Self.prefix) }.map(\.identifier))
        status = failures == 0 ? "Заплановано: \(pending.count)" : "Не вдалося запланувати: \(failures). Повтори перевірку."
    }
    func isScheduled(_ row: IOSReminder) -> Bool { pending.contains(Self.prefix + row.id.uuidString) }
}
