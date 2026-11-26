import SwiftUI
import UIKit
import UniformTypeIdentifiers

private let graphite = Color(red: 28/255, green: 28/255, blue: 30/255)
private let surface = Color(red: 43/255, green: 43/255, blue: 46/255)
private let crimson = Color(red: 238/255, green: 52/255, blue: 52/255)

struct CrimsonReticle: View {
    var state: CrimsonState = .idle
    var motion = true
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var scenePhase
    var body: some View {
        TimelineView(.animation(minimumInterval: state.animated ? 1.0/30.0 : 1.0/15.0, paused: !motion || reduceMotion || scenePhase != .active)) { timeline in
            let time = timeline.date.timeIntervalSinceReferenceDate
            let still = reduceMotion || !motion || scenePhase != .active
            let pulse = still ? 1.0 : 1.0 + sin(time * (state == .responding ? 5 : state == .thinking ? 3 : 1.8)) * (state == .responding ? 0.075 : 0.035)
            Canvas { context, size in
                let center = CGPoint(x: size.width/2, y: size.height/2)
                let r = min(size.width, size.height) * 0.27 * CGFloat(pulse)
                let circle = Path(ellipseIn: CGRect(x: center.x-r, y: center.y-r, width: 2*r, height: 2*r))
                context.stroke(circle, with: .color(crimson.opacity(state == .completed ? 0.24 : 0.10)), lineWidth: state == .completed ? 10 : 8)
                context.stroke(circle, with: .color(crimson), lineWidth: 2)
                for i in 0..<4 {
                    let a = CGFloat(i) * .pi/2
                    var line = Path()
                    line.move(to: CGPoint(x: center.x+cos(a)*r*0.68, y: center.y+sin(a)*r*0.68))
                    line.addLine(to: CGPoint(x: center.x+cos(a)*r*1.42, y: center.y+sin(a)*r*1.42))
                    context.stroke(line, with: .color(crimson), style: StrokeStyle(lineWidth: 2, lineCap: .round))
                }
                if state == .thinking {
                    for i in 0..<3 {
                        let start = still ? Double(i)*120 : time*85 + Double(i)*120
                        var arc = Path()
                        arc.addArc(center: center, radius: r*1.65, startAngle: .degrees(start), endAngle: .degrees(start+60), clockwise: false)
                        context.stroke(arc, with: .color(crimson.opacity(0.75)), lineWidth: 1.5)
                    }
                }
                if state == .responding {
                    let wave = r * (still ? 1.5 : 1.35 + 0.16 * CGFloat(sin(time * 5)))
                    context.stroke(Path(ellipseIn: CGRect(x: center.x-wave, y: center.y-wave, width: 2*wave, height: 2*wave)), with: .color(crimson.opacity(0.4)), lineWidth: 1.5)
                }
                if state == .failed || state == .cancelled {
                    var mark = Path()
                    mark.move(to: CGPoint(x: center.x-r*0.4,y: center.y+r*1.7))
                    mark.addLine(to: CGPoint(x: center.x+r*0.4,y: center.y+r*1.7))
                    context.stroke(mark, with: .color(crimson), style: StrokeStyle(lineWidth: 2, lineCap: .round))
                }
                context.fill(Path(ellipseIn: CGRect(x: center.x-2, y: center.y-2, width: 4, height: 4)), with: .color(crimson))
            }
        }
        .accessibilityHidden(true)
    }
}

@MainActor
struct FoundationView: View {
    @StateObject private var chat = ChatStore()
    @StateObject private var reminders = ReminderStore()
    @Environment(\.scenePhase) private var scenePhase
    @State private var input = ""
    @AppStorage("crimson.haptics") private var haptics = true
    @AppStorage("crimson.motion") private var motion = true
    @State private var followLatest = true
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @State private var settings = false
    @State private var endpoint = ""
    @State private var desktop = false
    @State private var sessionKey = ""
    @State private var model = ""
    @State private var settingsError = ""
    @State private var confirmServer = false
    @State private var confirmClear = false
    @State private var confirmRetry = false
    @State private var coreVersion = ""
    @State private var personality = CoreOptions()
    @State private var chooseGGUF = false
    @State private var confirmImport = false
    @State private var confirmRemove = false
    private let traitLabels = ["Аналітичність","Емпатія","Допитливість","Гумор","Ініціативність","Емоційність","Сарказм"]
    private let modeLabels = ["Companion":"Компаньйон","Assistant":"Асистент","Developer":"Розробник","Researcher":"Дослідник"]
    private let moodLabels = ["Calm":"Спокій","Joy":"Радість","Concern":"Турбота","Surprise":"Здивування","Curiosity":"Допитливість"]
    @FocusState private var focused: Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                CrimsonReticle(state: chat.feedbackState, motion: motion).frame(width: 44, height: 44)
                VStack(alignment: .leading, spacing: 3) {
                    Text("HATORIEL").font(.headline).tracking(2).lineLimit(1).minimumScaleFactor(0.7)
                    Text(chat.localMode ? "CRIMSON CORE · LOCAL" : chat.desktopMode ? "CRIMSON CORE · PC" : "CRIMSON CORE").font(.caption2).foregroundStyle(crimson).lineLimit(1).minimumScaleFactor(0.7)
                }
                Spacer()
                Button {
                    focused = false
                    endpoint = chat.endpoint; model = chat.model; desktop = chat.desktopMode; sessionKey = ""; personality = chat.options; settingsError = ""; settings = true
                } label: { Image(systemName: "gearshape").font(.title3).frame(width: 44, height: 44) }
                .accessibilityLabel("Налаштування").disabled(chat.busy)
            }.padding(.horizontal, 16).padding(.vertical, 8)
            Divider()
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 14) {
                        if chat.history.isEmpty {
                            CrimsonReticle(state: chat.feedbackState, motion: motion).frame(width: 148, height: 148).padding(.top, 24)
                            Text("Готова до діалогу").font(.title2.bold())
                            Text("Чим можу допомогти?").foregroundStyle(.secondary)
                        }
                        ForEach(chat.history) { entry in
                            bubble(role: entry.role, text: entry.content)
                        }
                        if chat.busy { bubble(role: "assistant", text: chat.partial.isEmpty ? "Думаю…" : chat.partial) }
                        Color.clear.frame(height: 1).id("end")
                    }.padding(16).frame(maxWidth: 780).frame(maxWidth: .infinity)
                }
                .scrollDismissesKeyboard(.interactively)
                .onAppear { proxy.scrollTo("end", anchor: .bottom) }
                .onChange(of: chat.history.count) { _ in if followLatest { proxy.scrollTo("end", anchor: .bottom) } }
                .onChange(of: chat.partial) { _ in if followLatest { proxy.scrollTo("end", anchor: .bottom) } }
                .onChange(of: chat.busy) { _ in if followLatest { proxy.scrollTo("end", anchor: .bottom) } }
                .safeAreaInset(edge: .bottom, spacing: 0) {
                    HStack {
                        Spacer()
                        Button {
                            followLatest.toggle()
                            if followLatest { proxy.scrollTo("end", anchor: .bottom) }
                        } label: {
                            Label(followLatest ? "Стежити за відповіддю" : "До останнього", systemImage: followLatest ? "arrow.down.circle.fill" : "arrow.down.circle")
                                .font(.caption).padding(8)
                        }.accessibilityAddTraits(followLatest ? .isSelected : [])
                    }.padding(.horizontal, 12)
                }
            }
            Divider()
            if let item = chat.suggestion, !chat.busy {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Запам’ятати?").font(.headline)
                    Text(item.content).font(.subheadline).lineLimit(4)
                    HStack {
                        Button("Відхилити") { chat.dismissSuggestion() }
                        Spacer()
                        Button("Зберегти") {
                            do { try chat.approveSuggestion() }
                            catch { chat.status = error.localizedDescription }
                        }.buttonStyle(.borderedProminent)
                    }
                }.padding(12).background(surface)
            }
            VStack(spacing: 8) {
                HStack {
                    VStack(alignment: .leading, spacing: 3) { Text(chat.feedbackState.label).font(.caption.bold()).foregroundStyle(crimson); Text(chat.status).font(.caption).foregroundStyle(.secondary).lineLimit(3) }
                    Spacer()
                    if !chat.busy && !chat.history.isEmpty {
                        Button { confirmRetry = true } label: { Image(systemName: "arrow.clockwise").frame(width: 44, height: 44) }
                            .accessibilityLabel("Повторити останній запит")
                    }
                }
                HStack(alignment: .bottom, spacing: 8) {
                    TextField("Напиши повідомлення…", text: $input, axis: .vertical)
                        .lineLimit(1...5).focused($focused).padding(12)
                        .background(surface, in: RoundedRectangle(cornerRadius: 12))
                    Button {
                        if chat.busy { chat.cancel() }
                        else if chat.send(input) { CrimsonFeedback.send(enabled: haptics); input = ""; focused = false; followLatest = true }
                    } label: {
                        Image(systemName: chat.busy ? "stop.fill" : "arrow.up")
                            .font(.headline).frame(width: 46, height: 46)
                            .background(Color(red: 0.63, green: 0.08, blue: 0.1), in: RoundedRectangle(cornerRadius: 12))
                    }.accessibilityLabel(chat.busy ? "Зупинити" : "Надіслати")
                     .disabled(chat.importing || (!chat.busy && input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty))
                }
            }.padding(12).frame(maxWidth: 780).frame(maxWidth: .infinity)
        }.background(graphite.ignoresSafeArea()).foregroundStyle(.white).tint(crimson)
        .sheet(isPresented: $settings, onDismiss: { sessionKey = "" }) { settingsView }
        .onChange(of: chat.feedbackRevision) { _ in if scenePhase == .active { CrimsonFeedback.finish(chat.feedbackState, enabled: haptics) } }
        .alert("Повторити останній запит?", isPresented: $confirmRetry) {
            Button("Скасувати", role: .cancel) {}
            Button("Повторити") { chat.retry() }
        } message: { Text("Попередня відповідь збережеться, якщо нова генерація не завершиться.") }
        .onChange(of: scenePhase) { phase in if phase == .background { chat.cancel() }; if phase == .active { chat.reloadMemory(); Task { await reminders.refresh() } } }
        .task { coreVersion = (try? NativeCore.version()) ?? "Недоступне"; await reminders.refresh() }
    }

    private func bubble(role: String, text: String) -> some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack {
                Text(role == "user" ? "ТИ" : "HATORIEL").font(.caption.bold())
                    .foregroundStyle(role == "user" ? Color.secondary : crimson)
                Spacer()
                Button {
                    UIPasteboard.general.string = text
                } label: { Image(systemName: "doc.on.doc").font(.caption).frame(width: 44, height: 44) }
                    .accessibilityLabel("Копіювати повідомлення")
            }
            Text(text).frame(maxWidth: .infinity, alignment: .leading).textSelection(.enabled)
        }.padding(12).background(role == "user" ? Color(red: 0.21, green: 0.15, blue: 0.15) : surface, in: RoundedRectangle(cornerRadius: 12))
    }

    private var settingsView: some View {
        NavigationStack {
            Form {
                Section("Crimson Core") {
                    Toggle("Тактильний відгук", isOn: $haptics)
                    Toggle("Анімація аватара", isOn: $motion)
                    Text("Системне обмеження руху завжди має пріоритет. Тактильний відгук залежить від пристрою та налаштувань iOS.").font(.footnote)
                }
                Section("Підключення") {
                    TextField("https://server.example", text: $endpoint).textContentType(.URL)
                        .keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled()
                    TextField("Модель", text: $model).textInputAutocapitalization(.never).autocorrectionDisabled()
                    Toggle("Підключення до Hatoriel на ПК", isOn: $desktop)
                    if desktop {
                        SecureField("Новий ключ сеансу (64 символи)", text: $sessionKey).textInputAutocapitalization(.never).autocorrectionDisabled()
                        Text("На ПК відкрий «Пристрої», запусти шлюз та скопіюй ключ. Потрібен окремий HTTPS-проксі в приватній мережі. Ключ зберігається лише в оперативній пам’яті; після перезапуску введи його знову.").font(.footnote)
                    }
                    if chat.desktopMode {
                        LabeledContent("Збережений сеанс", value: chat.hasDesktopKey ? "Ключ у пам’яті" : "Без ключа")
                        Text(chat.connectionStatus).font(.footnote)
                        Button("Перевірити збережене підключення") { Task { await chat.checkDesktop() } }.disabled(!chat.hasDesktopKey || chat.checkingConnection)
                        Button("Видалити ключ сеансу", role: .destructive) { chat.forgetDesktopKey(); sessionKey = "" }.disabled(chat.checkingConnection)
                    }
                    Text("Лише HTTPS із довіреним сертифікатом. Повідомлення та частина історії надсилаються на вибраний сервер Ollama. Для локального режиму скористайся розділом «Локальний AI».")
                        .font(.footnote)
                    if !settingsError.isEmpty { Text(settingsError).foregroundStyle(crimson) }
                    Button("Зберегти підключення") { confirmServer = true }.disabled(chat.checkingConnection)
                }
                Section("Локальний AI · Preview") {
                    Toggle("Локальна генерація", isOn: Binding(get: { chat.localMode }, set: { chat.configureLocal(enabled: $0, profile: chat.localProfile) }))
                        .disabled(chat.importing)
                    Picker("Профіль", selection: Binding(get: { chat.localProfile }, set: { chat.configureLocal(enabled: chat.localMode, profile: $0) })) {
                        ForEach(LocalProfile.allCases, id: \.self) { profile in Text(profile.rawValue).tag(profile) }
                    }.disabled(chat.importing)
                    LabeledContent("Модель", value: chat.localSize > 0 ? ByteCountFormatter.string(fromByteCount: chat.localSize, countStyle: .binary) : "Не імпортовано")
                    LabeledContent("Контекст", value: "\(chat.localProfile.context) токенів")
                    LabeledContent("Потоки CPU", value: "\(chat.localProfile.threads)")
                    LabeledContent("Оцінка RAM", value: chat.localSize > 0 ? ByteCountFormatter.string(fromByteCount: LocalModel.estimate(chat.localSize, profile: chat.localProfile), countStyle: .binary) : "—")
                    LabeledContent("Бюджет RAM", value: ByteCountFormatter.string(fromByteCount: LocalModel.budget, countStyle: .binary))
                    Button(chat.importing ? "Імпорт…" : "Імпортувати GGUF") { confirmImport = true }.disabled(chat.importing)
                    Button("Видалити локальну копію", role: .destructive) { confirmRemove = true }.disabled(chat.importing || chat.localSize == 0)
                    Text("CPU Preview. До 512 токенів відповіді. Оцінка RAM приблизна, сумісність не гарантована. У локальному режимі запити не надсилаються на сервер.").font(.footnote)
                    Text(chat.status).font(.caption)
                }
                Section("Завдання") {
                    NavigationLink {
                        ReminderView(store: reminders)
                    } label: {
                        LabeledContent("Нагадування та сповіщення", value: "\(reminders.data.reminders.count)")
                    }
                }
                Section("Перенесення між пристроями") {
                    NavigationLink("Захищений пакет") { SyncView(chat: chat) }
                }
                Section("Довготривала пам’ять") {
                    NavigationLink {
                        MemoryView(chat: chat)
                    } label: {
                        LabeledContent("Пам’ять і профіль", value: "\(chat.memory.records.count)/100")
                    }
                }
                Section("Особистість") {
                    Picker("Режим", selection: $personality.mode) {
                        ForEach(CoreOptions.modes, id: \.self) { mode in
                            Text(modeLabels[mode] ?? mode).tag(mode)
                        }
                    }
                    ForEach(0..<7, id: \.self) { i in
                        VStack(alignment: .leading, spacing: 5) {
                            HStack {
                                Text(traitLabels[i])
                                Spacer()
                                Text("\(personality.traits[i]) / 100").font(.caption.monospacedDigit())
                            }
                            Slider(value: Binding(get: { Double(personality.traits[i]) },
                                                  set: { personality.traits[i] = UInt8(max(0, min(100, $0.rounded()))) }),
                                   in: 0...100, step: 1)
                                .accessibilityLabel(traitLabels[i])
                        }
                    }
                    Toggle("Адаптивний стиль", isOn: $personality.dynamic_style)
                    Toggle("Симуляція емоцій", isOn: $personality.emotion_enabled)
                    if personality.emotion_enabled {
                        LabeledContent("Останній стан", value: moodLabels[personality.mood.mood] ?? "Спокій")
                        ProgressView(value: Double(personality.mood.intensity), total: 100)
                        Button("Скинути стан") { personality.mood = CoreMood() }
                        Text("Моделювання тону відповіді за правилами, а не справжні почуття. Стан уточнюється при наступному запиті.").font(.footnote)
                    }
                    if chat.effectiveTraits.count == 7 {
                        DisclosureGroup("Ефективні риси останнього контексту") {
                            ForEach(0..<7, id: \.self) { i in
                                LabeledContent(traitLabels[i], value: "\(chat.effectiveTraits[i])")
                            }
                        }
                    }
                    Text("Налаштування особистості використовуються у контексті, який надсилається підтвердженому AI-серверу.").font(.footnote)
                    Button("Відновити базові риси") { personality = CoreOptions() }
                    Button("Зберегти особистість") { chat.configurePersonality(personality); personality = chat.options }
                    Text(chat.status).font(.caption).foregroundStyle(.secondary)
                }
                Section("Дані") {
                    Button("Очистити історію", role: .destructive) { confirmClear = true }
                    Text("Історія зберігається у локальних налаштуваннях застосунку. Додаткового шифрування немає.").font(.footnote)
                }
                Section("Про Hatoriel") {
                    Text("Crimson Core Minimal · 0.5.9")
                    Text("Rust Core " + coreVersion)
                }
            }
            .fileImporter(isPresented: $chooseGGUF, allowedContentTypes: [.data]) { result in
                switch result {
                case .success(let url): chat.importModel(url)
                case .failure(let error): chat.status = error.localizedDescription
                }
            }
            .confirmationDialog("Імпортувати модель?", isPresented: $confirmImport, titleVisibility: .visible) {
                Button("Обрати GGUF") { chooseGGUF = true }
                Button("Скасувати", role: .cancel) {}
            } message: { Text("Нова копія замінить попередню тільки після успішного імпорту.") }
            .confirmationDialog("Видалити копію GGUF?", isPresented: $confirmRemove, titleVisibility: .visible) {
                Button("Видалити", role: .destructive) { chat.removeModel() }
                Button("Скасувати", role: .cancel) {}
            }
            .navigationTitle("Налаштування")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Готово") { settings = false } } }
            .alert("Підтвердити сервер?", isPresented: $confirmServer) {
                Button("Скасувати", role: .cancel) {}
                Button("Зберегти") {
                    do { try chat.configure(endpoint: endpoint, model: model, desktop: desktop, key: sessionKey); sessionKey = ""; settingsError = ""; if !desktop { settings = false } }
                    catch { settingsError = error.localizedDescription }
                }
            } message: { Text("Повідомлення та контекст будуть передаватися на " + endpoint) }
            .confirmationDialog("Видалити локальну історію?", isPresented: $confirmClear, titleVisibility: .visible) {
                Button("Видалити", role: .destructive) { chat.clear() }
                Button("Скасувати", role: .cancel) {}
            }
        }.preferredColorScheme(.dark).tint(crimson)
    }
}
