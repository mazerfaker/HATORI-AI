import SwiftUI

@MainActor struct MemoryView: View {
    @ObservedObject var chat: ChatStore
    @State private var search = ""
    @State private var filter = ""
    @State private var editing: Int64? = nil
    @State private var content = ""
    @State private var category = "General"
    @State private var importance = 50.0
    @State private var profile = CoreProfile()
    @State private var local = true
    @State private var remote = false
    @State private var suggestions = true
    @State private var consent = false
    @State private var removing: MemoryRecord?
    @State private var error = ""
    private var filtered: [MemoryRecord] {
        chat.memory.records.filter {
            (filter.isEmpty || $0.category == filter) &&
            (search.isEmpty || $0.content.localizedCaseInsensitiveContains(search))
        }.sorted { a,b in a.importance == b.importance ? a.id > b.id : a.importance > b.importance }
    }
    var body: some View {
        Form {
            if let failure = chat.memoryError {
                Section {
                    Text(failure).foregroundStyle(.red)
                    Button("Повторити завантаження") { chat.reloadMemory(); refresh() }
                }
            } else {
                Section("Конфіденційність") {
                    Toggle("Пам’ять у локальному режимі", isOn: $local)
                    Toggle("Пропонувати нові спогади", isOn: $suggestions)
                    Toggle("Дозволити передачу на AI-сервер", isOn: $remote)
                        .disabled(chat.endpoint.isEmpty)
                    Text("Передача вимкнена за замовчуванням. Дозвіл діє лише для поточної адреси сервера. Релевантні спогади й профіль додаються до контексту; звичайна історія чату передається окремо.").font(.footnote)
                    Button("Зберегти дозволи") {
                        if remote && !chat.memory.allows(local: false, endpoint: chat.endpoint) { consent = true }
                        else { savePermissions() }
                    }
                }
                Section("Підсумок розмови") {
                    Button("Запропонувати підсумок") { perform { try chat.proposeSummary() } }
                        .disabled(chat.busy || !chat.memory.suggestionsEnabled || chat.suggestion != nil)
                    if let item = chat.suggestion {
                        Text(item.content)
                        Button("Зберегти після перегляду") { perform { try chat.approveSuggestion() } }
                        Button("Відхилити", role: .cancel) { chat.dismissSuggestion() }
                    }
                }
                Section("Профіль користувача") {
                    TextField("Інтереси", text: $profile.interests, axis: .vertical)
                    TextField("Стиль спілкування", text: $profile.communication, axis: .vertical)
                    TextField("Цілі", text: $profile.goals, axis: .vertical)
                    Button("Зберегти профіль") { perform { try chat.saveProfile(profile) } }
                }
                Section(editing == nil ? "Новий спогад" : "Редагування") {
                    TextField("Що потрібно пам’ятати?", text: $content, axis: .vertical).lineLimit(2...5)
                    Picker("Категорія", selection: $category) {
                        ForEach(MemoryVault.categories, id: \.self) { c in Text(MemoryVault.labels[c] ?? c).tag(c) }
                    }
                    LabeledContent("Важливість", value: "\(Int(importance)) / 100")
                    Slider(value: $importance, in: 0...100, step: 1)
                    Button("Зберегти спогад") {
                        perform {
                            try chat.saveMemory(id: editing, content: content, category: category, importance: UInt8(importance))
                            resetEditor()
                        }
                    }.disabled(content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    if editing != nil { Button("Скасувати редагування") { resetEditor() } }
                }
                Section("Спогади · \(chat.memory.records.count)/100") {
                    TextField("Пошук", text: $search)
                    Picker("Фільтр", selection: $filter) {
                        Text("Усі").tag("")
                        ForEach(MemoryVault.categories, id: \.self) { c in Text(MemoryVault.labels[c] ?? c).tag(c) }
                    }
                    if filtered.isEmpty { Text("Немає спогадів").foregroundStyle(.secondary) }
                    ForEach(filtered) { row in
                        VStack(alignment: .leading, spacing: 8) {
                            Text(row.content)
                            Text("\(MemoryVault.labels[row.category] ?? row.category) · \(row.importance)/100").font(.caption).foregroundStyle(.secondary)
                            HStack {
                                Button("Редагувати") { editing = row.id; content = row.content; category = row.category; importance = Double(row.importance) }.buttonStyle(.borderless)
                                Spacer()
                                Button("Видалити", role: .destructive) { removing = row }.buttonStyle(.borderless)
                            }
                        }
                    }
                }
            }
            if !error.isEmpty { Section { Text(error).foregroundStyle(.red) } }
        }.navigationTitle("Пам’ять")
            .onAppear { refresh() }
            .alert("Дозволити передачу пам’яті?", isPresented: $consent) {
                Button("Скасувати", role: .cancel) { remote = false }
                Button("Дозволити") { savePermissions() }
            } message: { Text("Профіль і вибрані за релевантністю спогади надсилатимуться на " + chat.endpoint) }
            .confirmationDialog("Видалити спогад?", isPresented: Binding(get: { removing != nil }, set: { if !$0 { removing = nil } }), titleVisibility: .visible, presenting: removing) { row in
                Button("Видалити", role: .destructive) { perform { try chat.removeMemory(row.id); if editing == row.id { resetEditor() } }; removing = nil }
                Button("Скасувати", role: .cancel) { removing = nil }
            } message: { row in Text(row.content) }
    }
    private func refresh() {
        profile = chat.memory.profile; local = chat.memory.localEnabled
        suggestions = chat.memory.suggestionsEnabled
        remote = chat.memory.allows(local: false, endpoint: chat.endpoint)
    }
    private func resetEditor() { editing = nil; content = ""; category = "General"; importance = 50 }
    private func perform(_ action: () throws -> Void) {
        do { try action(); error = "" } catch { self.error = error.localizedDescription }
    }
    private func savePermissions() { perform { try chat.setMemoryOptions(local: local, suggestions: suggestions, remote: remote) } }
}
