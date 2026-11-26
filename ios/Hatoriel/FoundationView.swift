import SwiftUI

private let graphite = Color(red: 28/255, green: 28/255, blue: 30/255)
private let surface = Color(red: 43/255, green: 43/255, blue: 46/255)
private let crimson = Color(red: 238/255, green: 52/255, blue: 52/255)

struct CrimsonReticle: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var scenePhase
    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0/15.0, paused: reduceMotion || scenePhase != .active)) { timeline in
            let time = timeline.date.timeIntervalSinceReferenceDate
            let pulse = reduceMotion ? 1.0 : 1.0 + sin(time * 1.8) * 0.035
            Canvas { context, size in
                let center = CGPoint(x: size.width/2, y: size.height/2)
                let r = min(size.width, size.height) * 0.27 * pulse
                let circle = Path(ellipseIn: CGRect(x: center.x-r, y: center.y-r, width: 2*r, height: 2*r))
                context.stroke(circle, with: .color(crimson.opacity(0.1)), lineWidth: 8)
                context.stroke(circle, with: .color(crimson), lineWidth: 2)
                for i in 0..<4 {
                    let a = Double(i) * .pi/2
                    var line = Path()
                    line.move(to: CGPoint(x: center.x+cos(a)*r*0.68, y: center.y+sin(a)*r*0.68))
                    line.addLine(to: CGPoint(x: center.x+cos(a)*r*1.42, y: center.y+sin(a)*r*1.42))
                    context.stroke(line, with: .color(crimson), style: StrokeStyle(lineWidth: 2, lineCap: .round))
                }
                context.fill(Path(ellipseIn: CGRect(x: center.x-2, y: center.y-2, width: 4, height: 4)), with: .color(crimson))
            }
        }
        .accessibilityHidden(true)
    }
}

struct FoundationView: View {
    @State private var query = "Привіт, Hatoriel!"
    @State private var status = "Перевірка ядра"
    @State private var prepared: [CoreMessage] = []
    @State private var busy = false

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                HStack {
                    CrimsonReticle().frame(width: 44, height: 44)
                    VStack(alignment: .leading, spacing: 3) {
                        Text("H A T O R I E L").font(.headline)
                        Text("CRIMSON CORE").font(.caption2).foregroundStyle(crimson)
                    }
                    Spacer()
                    Text("0.5.0").font(.caption).foregroundStyle(.secondary)
                }
                CrimsonReticle().frame(width: 156, height: 156)
                Text("iOS Foundation").font(.title2.bold())
                Text("Діагностика спільного Rust-ядра. Генерація нейромережею ще не підключена.")
                    .font(.subheadline).foregroundStyle(.secondary).multilineTextAlignment(.center)
                Text(status).font(.footnote).frame(maxWidth: .infinity, alignment: .leading)
                TextField("Тестове повідомлення", text: $query, axis: .vertical)
                    .lineLimit(1...4).padding(12).background(surface, in: RoundedRectangle(cornerRadius: 12))
                Button(action: check) {
                    HStack {
                        if busy { ProgressView().tint(.white) }
                        Text("Перевірити Rust Core").fontWeight(.medium)
                    }.frame(maxWidth: .infinity).padding(13)
                }
                .buttonStyle(.plain).background(Color(red: 0.63, green: 0.08, blue: 0.1), in: RoundedRectangle(cornerRadius: 12))
                .disabled(busy || query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                if !prepared.isEmpty {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("Підготовлений контекст").font(.headline)
                        ForEach(Array(prepared.enumerated()), id: \.offset) { item in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(item.element.role.uppercased()).font(.caption).foregroundStyle(crimson)
                                Text(item.element.content).font(.footnote).textSelection(.enabled)
                            }
                        }
                    }.frame(maxWidth: .infinity, alignment: .leading).padding(14)
                        .background(surface, in: RoundedRectangle(cornerRadius: 12))
                }
            }.padding(20)
        }.background(graphite.ignoresSafeArea())
        .foregroundStyle(.white)
        .task {
            do { status = "Rust Core " + (try NativeCore.version()) }
            catch { status = error.localizedDescription }
        }
    }

    private func check() {
        guard !busy else { return }
        let value = query
        guard value.unicodeScalars.count <= 12000 else { status = "Максимум 12000 символів"; return }
        busy = true
        prepared = []
        Task { @MainActor in
            do {
                prepared = try await Task.detached(priority: .userInitiated) {
                    try NativeCore.prepare([CoreMessage(role: "user", content: value)])
                }.value
                status = "Контекст успішно підготовлено"
            } catch { status = error.localizedDescription }
            busy = false
        }
    }
}
