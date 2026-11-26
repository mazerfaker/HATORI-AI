# HATORI AI — Genesis 0.1.0

Нативний прототип Rust/egui. Windows 10/11. Не містить ваг моделі або готового EXE.

## Запуск
1. Встановити Rust stable MSVC (https://rustup.rs/) та Visual Studio Build Tools з Desktop development with C++.
2. Встановити Ollama (https://ollama.com/download/windows), запустити його.
3. У PowerShell: `ollama pull qwen2.5:3b`.
4. Запустити BUILD_WINDOWS.bat. Перший запуск Cargo потребує інтернету. EXE буде в target/release.
5. У Models можна вибрати іншу встановлену модель.

## Реалізовано у вихідному коді
- Нативний темний UI, анімоване ядро, потоковий чат через локальний Ollama.
- SQLite історія, ручне додавання спогадів, налаштування профілю.
- 4 режими як текстові інструкції моделі; не автономні агенти.
- UA/EN часткова локалізація; діагностика помилок у статусі.
- Профіль 90/80/90/90/90/80/70. Значення є підказками моделі, не вимірюваними емоціями.
- Дані: %LOCALAPPDATA%/HATORI_AI/memory.sqlite3. Не зашифровані. Спогади передаються локальній моделі. Для повного скидання закрити програму та видалити базу.

## QA / обмеження
Rust/Cargo та Windows недоступні у середовищі створення, тому компіляцію та runtime не перевірено. Cargo.lock ще не створено. Не заявляємо готову збірку. Немає голосу, автоматичного навчання, моніторингу ресурсів, керування ОС, файлового журналу, видалення окремих спогадів. Модель не отримує інструментів виконання команд. Перевірити cargo check, запуск, кирилицю, відповіді, відновлення історії, помилки при вимкненому Ollama. Після встановлення моделі чат може працювати без інтернету.

## v0.1.1 — Stability & Desktop Presence
- Windows tray: Open, Toggle avatar, Exit; double-click restores. Minimize/close hides when enabled and tray initialization succeeds.
- Separate transparent always-on-top animated geometric avatar, draggable, Open/Hide buttons. Toggle persisted in SQLite. Existing config migrated via serde defaults.
- Model-list errors no longer reset chat generation state.
- Windows compilation and runtime behavior are NOT verified; Rust toolchain unavailable in development environment. Test tray restore with avatar disabled, close/minimize, exit, restart settings, and generation while hidden. Avatar is procedural placeholder, not final character art.
- Update by copying Cargo.toml and src files over previous project; keep Cargo.lock and target if present. Cargo may update lock for new dependency.
