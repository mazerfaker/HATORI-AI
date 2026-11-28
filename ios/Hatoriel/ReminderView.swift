import SwiftUI
import UIKit

@MainActor struct ReminderView: View {
    @ObservedObject var store: ReminderStore
    @State private var enabled = false
    @State private var quiet = false
    @State private var sound = true
    @State private var showText = false
    @State private var checkIn = false
    @State private var interval = 120
    @State private var text = ""
    @State private var minutes = "30"
    @State private var removing: IOSReminder?
    @State private var confirmPreview = false
    @Environment(\.openURL) private var openURL
    var body: some View {
        Form {
            Section("Дозволи iOS") {
                LabeledContent("Сповіщення", value: store.authorizationLabel)
                Button("Запросити дозвіл") { Task { await store.requestPermission() } }
                    .disabled(store.working || store.allowed || store.authorization == .denied)
                Button("Відкрити системні налаштування") {
                    if let url = URL(string: UIApplication.openSettingsURLString) { openURL(url) }
                }
                Button("Оновити стан") { Task { await store.refresh(); loadSettings() } }.disabled(store.working)
                if let error = store.storageError { Text(error).foregroundStyle(.red) }
                Text(store.status).font(.footnote).foregroundStyle(.secondary)
            }
            Section("Керування") {
                Toggle("Локальні сповіщення", isOn: $enabled)
                Toggle("Режим тиші", isOn: $quiet)
                Toggle("Звук", isOn: $sound)
                Toggle("Показувати текст у сповіщенні", isOn: $showText)
                Text("Без показу тексту сповіщення містить лише загальне повідомлення. iOS керує вібрацією, Focus і доставкою. Режим тиші скасовує майбутні системні нагадування; прострочені не повторюються.").font(.footnote)
                Toggle("Періодично нагадувати про цілі", isOn: $checkIn)
                if checkIn {
                    Stepper("Кожні \(interval) хв", value: $interval, in: 60...1440, step: 60)
                    Text("Це шаблонне локальне нагадування, без фонової генерації та передачі даних.").font(.footnote)
                }
                Button("Зберегти параметри") {
                    if showText && !store.data.showText { confirmPreview = true }
                    else { saveSettings() }
                }
            }.disabled(store.working || store.storageError != nil)
            Section("Нове нагадування") {
                TextField("Про що нагадати?", text: $text, axis: .vertical).lineLimit(2...4)
                TextField("Через скільки хвилин", text: $minutes).keyboardType(.numberPad)
                Button("Додати") {
                    guard let delay = Int(minutes) else { return }
                    let before = store.data.reminders.count
                    Task {
                        await store.add(text: text, minutes: delay)
                        if store.data.reminders.count > before { text = "" }
                    }
                }.disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || Int(minutes).map { !(1...10080).contains($0) } != false)
            }.disabled(store.working || store.storageError != nil)
            Section("Завдання · \(store.data.reminders.count)/40") {
                if store.data.reminders.isEmpty { Text("Немає нагадувань").foregroundStyle(.secondary) }
                ForEach(store.data.reminders.sorted { $0.due < $1.due }) { row in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(row.text)
                        Text(row.due, format: .dateTime.day().month().hour().minute()).font(.caption)
                        Text(store.isScheduled(row) ? "Заплановано в iOS" : row.due <= Date() ? "Час минув · доставка не підтверджена" : "Не заплановано").font(.caption).foregroundStyle(.secondary)
                        Button("Відкласти на 15 хв") { Task { await store.snooze(row.id, minutes: 15) } }.buttonStyle(.borderless)
                        Button("Видалити", role: .destructive) { removing = row }.buttonStyle(.borderless)
                    }
                }
            }.disabled(store.working || store.storageError != nil)
        }
        .navigationTitle("Нагадування")
        .task { await store.refresh(); loadSettings() }
        .alert("Показувати текст нагадувань?", isPresented: $confirmPreview) {
            Button("Скасувати", role: .cancel) { showText = false }
            Button("Дозволити") { saveSettings() }
        } message: { Text("Текст може бути видимим на екрані блокування залежно від системних налаштувань iOS.") }
        .confirmationDialog("Видалити нагадування?", isPresented: Binding(get: { removing != nil }, set: { if !$0 { removing = nil } }), titleVisibility: .visible, presenting: removing) { row in
            Button("Видалити", role: .destructive) { removing = nil; Task { await store.remove(row.id) } }
            Button("Скасувати", role: .cancel) { removing = nil }
        } message: { row in Text(row.text) }
    }
    private func loadSettings() {
        enabled = store.data.enabled; quiet = store.data.quiet; sound = store.data.sound
        showText = store.data.showText; checkIn = store.data.checkIn; interval = store.data.checkInMinutes
    }
    private func saveSettings() {
        Task { await store.saveSettings(enabled: enabled, quiet: quiet, sound: sound, showText: showText, checkIn: checkIn, minutes: interval); loadSettings() }
    }
}
