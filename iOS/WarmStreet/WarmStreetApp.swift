import SwiftUI

import SwiftUI

class AppDelegate: NSObject, UIApplicationDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey : Any]? = nil) -> Bool {
        return true
    }

    func application(_ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
        let token = deviceToken.map { String(format: "%02.2hhx", $0) }.joined()
        // Pass token back to core via PushHandler
        PushHandler.shared.core?.update(.pushTokenReceived(token))
    }

    func application(_ application: UIApplication, didFailToRegisterForRemoteNotificationsWithError error: Error) {
        print("Failed to register: \(error)")
    }
}

@main
struct WarmStreetApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @StateObject private var core = Core()

    init() {
        PushHandler.shared.core = core
    }

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
                .onAppear {
                    PushHandler.shared.core = core
                }
        }
    }
}
