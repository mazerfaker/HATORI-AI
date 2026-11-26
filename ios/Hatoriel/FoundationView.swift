import SwiftUI
import UIKit

private let graphite = Color(red: 28/255, green: 28/255, blue: 30/255)
private let surface = Color(red: 43/255, green: 43/255, blue: 46/255)
private let crimson = Color(red: 238/255, green: 52/255, blue: 52/255)

struct CrimsonReticle: View {
    var busy = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var scenePhase
    var body: some View {
        TimelineView(.animation(minimumInterval: busy ? 1.0/30.0 : 1.0/15.0, paused: reduceMotion || scenePhase != .active)) { timeline in
            let time = timeline.date.timeIntervalSinceReferenceDate
            let pulse = reduceMotion ? 1.0 : 1.0 + sin(time * (busy ? 4 : 1.8)) * 0.035
            Canvas { context, size in
                let center = CGPoint(x: size.width/2, y: size.height/2)
                let r = min(size.width, size.height) * 0.27 * CGFloat(pulse)
                let circle = Path(ellipseIn: CGRect(x: center.x-r, y: center.y-r, width: 2*r, height: 2*r))
                context.stroke(circle, with: .color(crimson.opacity(0.1)), lineWidth: 8)
                context.stroke(circle, with: .color(crimson), lineWidth: 2)
                for i in 0..<4 {
                    let a = CGFloat(i) * .pi/2
                    var line = Path()
                    line.move(to: CGPoint(x: center.x+cos(a)*r*0.68, y: center.y+sin(a)*r*0.68))
                    line.addLine(to: CGPoint(x: center.x+cos(a)*r*1.42, y: center.y+sin(a)*r*1.42))
                    context.stroke(line, with: .color(crimson), style: StrokeStyle(lineWidth: 2, lineCap: .round))
                }
                if busy {
                    for i in 0..<3 {
                        let start = reduceMotion ? Double(i)*120 : time*85 + Double(i)*120
                        var arc = Path()
                        arc.addArc(center: center, radius: r*1.65, startAngle: .degrees(start), endAngle: .degrees(start+60), clockwise: false)
                        context.stroke(arc, with: .color(crimson.opacity(0.75)), lineWidth: 1.5)
                    }
                }
                context.fill(Path(ellipseIn: CGRect(x: center.x-2, y: center.y-2, width: 4, height: 4)), with: .color(crimson))
            }
        }
        .accessibilityHidden(true)
    }
}

struct FoundationView: View {
    @StateObject private var chat = ChatStore()
    @Environment(\.scenePhase) private var scenePhase
    @State private var input = ""
    @State private var settings = false
    @State private var endpoint = ""
    @State private var model = ""
    @State private var settingsError = ""
    @State private var confirmServer = false
    @State private var confirmClear = false
    @State private var confirmRetry = false
    @State private var coreVersion = ""
    @FocusState private var focused: Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                CrimsonReticle(busy: chat.busy).frame(width: 44, height: 44)
                VStack(alignment: .leading, spacing: 3) {
                    Text("H A T O R I E L").font(.headline)
                    Text("CRIMSON CORE").font(.caption2).foregroundStyle(crimson)
                }
                Spacer()
                Button {
                    focused = false
                    endpoint = chat.endpoint; model = chat.model; settingsError = ""; settings = true
                } label: { Image(systemName: "gearshape").font(.title3).frame(width: 44, height: 44) }
                .accessibilityLabel("Налаштування").disabled(chat.busy)
            }.padding(.horizontal, 16).padding(.vertical, 8)
            Divider()
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 14) {
                        if chat.history.isEmpty {
                            CrimsonReticle().frame(width: 148, height: 148).padding(.top, 24)
                            Text("Готова до діалогу").font(.title2.bold())
                            Text("Чим можу допомогти?").foregroundStyle(.secondary)
                        }
                        ForEach(chat.history) { entry in
                            bubble(role: entry.role, text: entry.content)
                        }
                        if chat.busy { bubble(role: "assistant", text: chat.partial.isEmpty ? "Думаю…" : chat.partial) }
                        Color.clear.frame(height: 1).id("end")
                    }.padding(16)
                }
                .scrollDismissesKeyboard(.interactively)
                .onAppear { proxy.scrollTo("end", anchor: .bottom) }
                .onChange(of: chat.history.count) { _ in proxy.scrollTo("end", anchor: .bottom) }
                .onChange(of: chat.partial) { _ in proxy.scrollTo("end", anchor: .bottom) }
                .onChange(of: chat.busy) { _ in proxy.scrollTo("end", anchor: .bottom) }
            }
            Divider()
            VStack(spacing: 8) {
                HStack {
                    Text(chat.status).font(.caption).foregroundStyle(.secondary).lineLimit(3)
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
                        else if chat.send(input) { input = ""; focused = false }
                    } label: {
                        Image(systemName: chat.busy ? "stop.fill" : "arrow.up")
                            .font(.headline).frame(width: 46, height: 46)
                            .background(Color(red: 0.63, green: 0.08, blue: 0.1), in: RoundedRectangle(cornerRadius: 12))
                    }.accessibilityLabel(chat.busy ? "Зупинити" : "Надіслати")
                     .disabled(!chat.busy && input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }.padding(12)
        }.background(graphite.ignoresSafeArea()).foregroundStyle(.white).tint(crimson)
        .sheet(isPresented: $settings) { settingsView }
        .alert("Повторити останній запит?", isPresented: $confirmRetry) {
            Button("Скасувати", role: .cancel) {}
            Button("Повторити") { chat.retry() }
        } message: { Text("Попередня відповідь збережеться, якщо нова генерація не завершиться.") }
        .onChange(of: scenePhase) { phase in if phase == .background { chat.cancel() } }
        .task { coreVersion = (try? NativeCore.version()) ?? "Недоступне" }
    }

    private func bubble(role: String, text: String) -> some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack {
                Text(role == "user" ? "ТИ" : "HATORIEL").font(.caption.bold())
                    .foregroundStyle(role == "user" ? Color.secondary : crimson)
                Spacer()
                Button {
                    UIPasteboard.general.string = text
                } label: { Image(systemName: "doc.on.doc").font(.caption).frame(width: 44, height: 36) }
                    .accessibilityLabel("Копіювати повідомлення")
            }
            Text(text).frame(maxWidth: .infinity, alignment: .leading).textSelection(.enabled)
        }.padding(12).background(role == "user" ? Color(red: 0.21, green: 0.15, blue: 0.15) : surface, in: RoundedRectangle(cornerRadius: 12))
    }

    private var settingsView: some View {
        NavigationStack {
            Form {
                Section("Підключення") {
                    TextField("https://server.example", text: $endpoint).textContentType(.URL)
                        .keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled()
                    TextField("Модель", text: $model).textInputAutocapitalization(.never).autocorrectionDisabled()
                    Text("Лише HTTPS із довіреним сертифікатом. Повідомлення та частина історії надсилаються на вибраний сервер Ollama. Локальний GGUF ще не підключено.")
                        .font(.footnote)
                    if !settingsError.isEmpty { Text(settingsError).foregroundStyle(crimson) }
                    Button("Зберегти підключення") { confirmServer = true }
                }
                Section("Дані") {
                    Button("Очистити історію", role: .destructive) { confirmClear = true }
                    Text("Історія зберігається у локальних налаштуваннях застосунку. Додаткового шифрування немає.").font(.footnote)
                }
                Section("Про Hatoriel") {
                    Text("Crimson Core Minimal · 0.5.1")
                    Text("Rust Core " + coreVersion)
                }
            }
            .navigationTitle("Налаштування")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Готово") { settings = false } } }
            .alert("Підтвердити сервер?", isPresented: $confirmServer) {
                Button("Скасувати", role: .cancel) {}
                Button("Зберегти") {
                    do { try chat.configure(endpoint: endpoint, model: model); settingsError = ""; settings = false }
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
