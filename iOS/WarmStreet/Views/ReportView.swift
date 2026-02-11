import SwiftUI
import Shared

struct ReportView: View {
    @EnvironmentObject var core: Core
    @Environment(\.dismiss) var dismiss
    
    @State private var description: String = ""
    @State private var woundSeverity: Double = 1.0
    
    var body: some View {
        NavigationView {
            ZStack {
                // background gradient
                LinearGradient(colors: [.blue.opacity(0.1), .purple.opacity(0.1)], startPoint: .top, endPoint: .bottom)
                    .ignoresSafeArea()
                
                ScrollView {
                    VStack(spacing: 24) {
                        // Image Preview with Glass Card
                        ZStack(alignment: .topTrailing) {
                            if let imgData = displayImageData, let uiImage = UIImage(data: Data(imgData)) {
                                Image(uiImage: uiImage)
                                    .resizable()
                                    .scaledToFill()
                                    .frame(height: 250)
                                    .clipShape(RoundedRectangle(cornerRadius: 16))
                            } else {
                                RoundedRectangle(cornerRadius: 16)
                                    .fill(Color.gray.opacity(0.1))
                                    .frame(height: 250)
                                    .overlay(
                                        VStack(spacing: 12) {
                                            Image(systemName: "camera.shutter.button")
                                                .font(.largeTitle)
                                            Text("No photo captured")
                                        }
                                        .foregroundColor(.gray)
                                    )
                            }
                            
                            // AI Badge
                            if core.view.detectionCount > 0 {
                                HStack(spacing: 6) {
                                    Image(systemName: "pawprint.fill")
                                        .symbolEffect(.bounce, value: core.view.detectionCount)
                                    Text("Animal Detected")
                                        .font(.system(size: 14, weight: .bold, design: .rounded))
                                    Text("\(Int(core.view.topConfidence * 100))%")
                                        .font(.system(size: 12, weight: .medium, design: .monospaced))
                                        .padding(.horizontal, 6)
                                        .background(Color.white.opacity(0.2))
                                        .cornerRadius(4)
                                }
                                .padding(.horizontal, 12)
                                .padding(.vertical, 8)
                                .glassStyle(cornerRadius: 20, fill: .green.opacity(0.6))
                                .foregroundColor(.white)
                                .padding(12)
                                .transition(.scale.combined(with: .opacity))
                            }
                        }
                        .glassStyle(cornerRadius: 16)
                        
                        // Inputs Card
                        VStack(alignment: .leading, spacing: 20) {
                            VStack(alignment: .leading, spacing: 10) {
                                Label("Description", systemImage: "text.alignleft")
                                    .font(.subheadline.bold())
                                    .foregroundColor(.secondary)
                                
                                TextEditor(text: $description)
                                    .frame(height: 80)
                                    .scrollContentBackground(.hidden)
                                    .background(Color.white.opacity(0.1))
                                    .cornerRadius(8)
                            }
                            
                            Divider()
                            
                            VStack(alignment: .leading, spacing: 12) {
                                HStack {
                                    Label("Wound Severity", systemImage: "cross.case.fill")
                                        .font(.subheadline.bold())
                                        .foregroundColor(.secondary)
                                    Spacer()
                                    Text("\(Int(woundSeverity))")
                                        .font(.system(.title3, design: .rounded).bold())
                                        .foregroundColor(colorForSeverity(Int(woundSeverity)))
                                }
                                
                                Slider(value: $woundSeverity, in: 1...5, step: 1)
                                    .accentColor(colorForSeverity(Int(woundSeverity)))
                            }
                        }
                        .padding(20)
                        .glassStyle(cornerRadius: 20)
                        
                        Spacer(minLength: 40)
                        
                        // Submit Button
                        Button(action: {
                            UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                            submitCase()
                        }) {
                            HStack {
                                Text("Report to WarmStreet")
                                Image(systemName: "arrow.up.right.circle.fill")
                            }
                            .font(.headline)
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(
                                LinearGradient(colors: [.blue, .cyan], startPoint: .leading, endPoint: .trailing)
                            )
                            .clipShape(Capsule())
                            .shadow(color: .blue.opacity(0.3), radius: 10, x: 0, y: 5)
                        }
                    }
                    .padding(20)
                }
            }
            .navigationTitle("New Report")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        core.update(.photoCancelled)
                        dismiss()
                    }
                }
            }
        }
    }
    
    private var displayImageData: [UInt8]? {
        // Prefer crop if available
        return core.view.stagedCrop ?? core.view.stagedPhoto
    }
    
    private func colorForSeverity(_ val: Int) -> Color {
        switch val {
        case 1: return .green
        case 2: return .yellow
        case 3: return .orange
        case 4: return .red
        case 5: return .purple
        default: return .gray
        }
    }
    
    private func submitCase() {
        // Get current location from core later or assume it's already in model flow if needed.
        // For now, we need to pass a location to CreateCasePayload.
        // We'll use (0,0) or last drop pin if available.
        // Ideally the core knows the location.
        
        let lat = core.view.areaCenter?.0 ?? 0.0
        let lng = core.view.areaCenter?.1 ?? 0.0
        
        let payload = CreateCasePayload(
            location: (lat, lng),
            description: description.isEmpty ? nil : description,
            woundSeverity: Int32(woundSeverity)
        )
        
        core.update(.createCaseRequested(payload))
        dismiss()
    }
}
