import SwiftUI
import UIKit
import UserNotifications

final class HatorielAppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        UNUserNotificationCenter.current().delegate = self
        return true
    }
    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification, withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        var options: UNNotificationPresentationOptions = [.banner, .list]
        if notification.request.content.sound != nil { options.insert(.sound) }
        completionHandler(options)
    }
}


@main
struct HatorielApp: App {
    @UIApplicationDelegateAdaptor(HatorielAppDelegate.self) private var delegate
    var body: some Scene {
        WindowGroup {
            FoundationView()
                .preferredColorScheme(.dark)
        }
    }
}
