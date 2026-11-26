import SwiftUI
import UIKit
struct SyncRecord: Codable, Sendable { let content: String; let category: String; let importance: UInt8 }
struct SyncSnapshot: Codable, Sendable { let version: Int; let records: [SyncRecord]; let profile: CoreProfile; let options: CoreOptions }
struct SyncSeal: Encodable, Sendable { let action = "seal"; let snapshot: SyncSnapshot; let password: String }
struct SyncPreview: Encodable, Sendable { let action = "preview"; let local: SyncSnapshot; let package: String; let password: String }
struct SyncPackage: Decodable, Sendable { let package: String }
struct SyncPlan: Decodable, Sendable { let additions: [SyncRecord]; let duplicates: Int; let conflicts: Int; let profile: CoreProfile; let options: CoreOptions }

@MainActor struct SyncView: View {
    @ObservedObject var chat: ChatStore
    @State private var password = ""
    @State private var package = ""
    @State private var plan: SyncPlan?
    @State private var baseline: Data?
    @State private var profile = false
    @State private var personality = false
    @State private var working = false
    @State private var status = ""
    var body: some View {
        Form {
            Section("Захищене перенесення") {
                Text("Пакет містить спогади, профіль та налаштування особистості. Не містить історії, ключів серверів або дозволів передачі пам’яті. Пароль передавай окремо.")
                    .font(.footnote)
                SecureField("Парольна фраза (12+ символів)", text: $password)
                TextEditor(text: $package).disabled(working).frame(minHeight: 100).autocorrectionDisabled().textInputAutocapitalization(.never)
                Button("Створити зашифрований пакет") { run(export: true) }.disabled(working)
                Button("Копіювати пакет") { UIPasteboard.general.string = package }.disabled(package.isEmpty || working)
                Button("Переглянути імпорт") { run(export: false) }.disabled(package.isEmpty || working)
                if working { ProgressView() }
                Text(status).font(.footnote)
            }
            if let plan {
                Section("Попередній перегляд") {
                    LabeledContent("Нові спогади", value: String(plan.additions.count))
                    LabeledContent("Дублікати", value: String(plan.duplicates))
                    LabeledContent("Конфлікти (локальні збережено)", value: String(plan.conflicts))
                    ForEach(Array(plan.additions.enumerated()), id: \.offset) { row in
                        VStack(alignment: .leading) { Text(row.element.category + " · " + String(row.element.importance) + "/100").font(.caption).foregroundStyle(.secondary); Text(row.element.content).font(.footnote) }
                    }
                    DisclosureGroup("Вхідний профіль") {
                        LabeledContent("Інтереси", value: plan.profile.interests)
                        LabeledContent("Спілкування", value: plan.profile.communication)
                        LabeledContent("Цілі", value: plan.profile.goals)
                    }
                    DisclosureGroup("Вхідна особистість") {
                        Text(plan.options.mode)
                        Text(plan.options.traits.map { String($0) }.joined(separator: " · "))
                        LabeledContent("Адаптивний стиль", value: plan.options.dynamic_style ? "Так" : "Ні")
                        LabeledContent("Емоції", value: plan.options.emotion_enabled ? "Так" : "Ні")
                    }
                    Toggle("Замінити профіль", isOn: $profile)
                    Toggle("Замінити особистість", isOn: $personality)
                    Button("Підтвердити об’єднання") {
                        do {
                            guard let baseline, baseline == (try JSONEncoder.sorted.encode(chat.syncSnapshot())) else { throw CoreFailure.rejected("Локальні дані змінилися. Повтори перегляд.") }
                            try chat.applySync(plan, profile: profile, personality: personality)
                            self.plan = nil; self.baseline = nil; status = "Імпортовано"
                        } catch { status = error.localizedDescription }
                    }.disabled(working)
                }
            }
        }.navigationTitle("Перенесення")
        .onChange(of: package) { _ in plan = nil; baseline = nil }
        .onDisappear { password = "" }
    }
    private func run(export: Bool) {
        guard !working else { return }
        do {
            let snapshot = try chat.syncSnapshot()
            let pass = password, text = package
            let original = try JSONEncoder.sorted.encode(snapshot)
            working = true; plan = nil; baseline = nil; profile = false; personality = false; status = ""
            password = ""
            Task {
                do {
                    if export {
                        let result = try await Task.detached { try NativeCore.sync(SyncSeal(snapshot: snapshot, password: pass), as: SyncPackage.self) }.value
                        package = result.package; status = "Пакет створено"
                    } else {
                        let result = try await Task.detached { try NativeCore.sync(SyncPreview(local: snapshot, package: text, password: pass), as: SyncPlan.self) }.value
                        if package == text { plan = result; baseline = original; status = "Перевір зміни перед імпортом" }
                    }
                } catch { status = error.localizedDescription }
                working = false
            }
        } catch { status = error.localizedDescription }
    }
}
private extension JSONEncoder {
    static var sorted: JSONEncoder { let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys]; return encoder }
}
