import SwiftUI
import Shared

struct LoginView: View {
    @EnvironmentObject var core: Core
    @State private var isLoading = false
    private let authService = AuthService()
    
    var body: some View {
        ZStack {
            // Animated Gradient Background
            LinearGradient(colors: [Color(hex: "0D1117"), Color(hex: "1C1C1E"), Color(hex: "0D1117")], 
                           startPoint: .topLeading, endPoint: .bottomTrailing)
                .ignoresSafeArea()
            
            // Decorative Blur Orbs
            Circle()
                .fill(Color.blue.opacity(0.15))
                .frame(width: 300, height: 300)
                .blur(radius: 80)
                .offset(x: -150, y: -200)
            
            Circle()
                .fill(Color.purple.opacity(0.15))
                .frame(width: 300, height: 300)
                .blur(radius: 80)
                .offset(x: 150, y: 200)

            VStack(spacing: 40) {
                Spacer()
                
                // Premium Logo Aesthetic
                VStack(spacing: 8) {
                    Image(systemName: "shield.lefthalf.filled")
                        .font(.system(size: 80))
                        .foregroundStyle(
                            .linearGradient(colors: [.blue, .cyan], startPoint: .topLeading, endPoint: .bottomTrailing)
                        )
                        .shadow(color: .blue.opacity(0.5), radius: 20)
                    
                    Text("WarmStreet")
                        .font(.system(size: 42, weight: .black, design: .rounded))
                        .foregroundColor(.white)
                    
                    Text("COMMUNITY ANIMAL RESCUE")
                        .font(.caption2.bold())
                        .kerning(4)
                        .foregroundColor(.blue.opacity(0.8))
                }
                
                Spacer()
                
                // Login Card
                VStack(spacing: 24) {
                    Text("Welcome Back")
                        .font(.headline)
                        .foregroundColor(.white.opacity(0.9))
                    
                    Button(action: signInWithGoogle) {
                        HStack(spacing: 12) {
                            Image(systemName: "globe.americas.fill")
                                .font(.title3)
                            Text("Continue with Google")
                                .fontWeight(.bold)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 16)
                        .background(
                            LinearGradient(colors: [.blue, .cyan], startPoint: .leading, endPoint: .trailing)
                        )
                        .foregroundColor(.white)
                        .clipShape(Capsule())
                        .shadow(color: .blue.opacity(0.3), radius: 10, x: 0, y: 5)
                    }
                    .disabled(isLoading)
                    
                    if isLoading {
                        ProgressView()
                            .tint(.white)
                    }
                }
                .padding(32)
                .glassStyle(cornerRadius: 30) // Liquid Glass Card
                
                Text("By continuing, you agree to our Terms of Service.")
                    .font(.caption2)
                    .foregroundColor(.white.opacity(0.4))
                    .padding(.bottom, 20)
            }
            .padding(24)
        }
    }
    
    func signInWithGoogle() {
        Task {
            isLoading = true
            // Example Neon Auth URL - replace with config
            let authUrl = URL(string: "https://your-neon-auth-url/login")!
            
            do {
                let callbackURL = try await authService.signIn(url: authUrl, callbackURLScheme: "warmstreet")
                // Extract JWT from callbackURL fragment or query
                // Assuming callback format: warmstreet://auth#access_token=...
                // Parsing logic simplified here:
                if let fragment = callbackURL.fragment {
                    // primitive parsing
                     let params = fragment.components(separatedBy: "&")
                     if let tokenParam = params.first(where: { $0.hasPrefix("access_token=") }) {
                         let jwt = String(tokenParam.dropFirst("access_token=".count))
                         // Dispatch to core
                         // Need User ID too - usually decode JWT payload here or core does it
                         core.update(.loginCompleted(jwt: jwt, userId: "extracted_sub")) // Placeholder extraction
                     }
                }
            } catch {
                print("Auth error: \(error)")
            }
            isLoading = false
        }
    }
}
