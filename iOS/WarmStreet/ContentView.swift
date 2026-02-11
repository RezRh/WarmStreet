import SwiftUI
import Shared

struct ContentView: View {
    @StateObject private var core = Core()
    
    var body: some View {
        Group {
            switch core.view.state {
            case .unauthenticated:
                LoginView()
            case .authenticating:
                ProgressView("Signing in...")
            case .onboardingLocation:
                LocationPermissionView()
            case .onboardingRadius:
                RadiusPickerView()
            case .ready:
                FeedView()
            }
        }
        .environmentObject(core)
    }
}
