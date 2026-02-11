import SwiftUI
import Shared

struct CaseDetailSheet: View {
    let detail: CaseDetail
    @EnvironmentObject var core: Core
    
    var body: some View {
            ZStack {
                LinearGradient(colors: [.blue.opacity(0.1), .purple.opacity(0.1)], startPoint: .top, endPoint: .bottom)
                    .ignoresSafeArea()
                
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        // Header Image with Glass Overlay
                        ZStack(alignment: .bottomTrailing) {
                            if let url = detail.photoUrl, let imageUrl = URL(string: url) {
                                AsyncImage(url: imageUrl) { phase in
                                    if let image = phase.image {
                                        image.resizable().aspectRatio(contentMode: .fill)
                                    } else {
                                        Color.gray.opacity(0.2)
                                    }
                                }
                                .frame(height: 250)
                                .clipped()
                                .cornerRadius(20)
                            } else {
                                RoundedRectangle(cornerRadius: 20)
                                    .fill(Color.gray.opacity(0.1))
                                    .frame(height: 250)
                                    .overlay(Image(systemName: "photo").font(.largeTitle).foregroundColor(.gray))
                            }
                            
                            // Status Badge
                            Text(detail.status.replacingOccurrences(of: "_", with: " ").capitalized)
                                .font(.caption.bold())
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                                .glassStyle(cornerRadius: 12, fill: .blue.opacity(0.6))
                                .foregroundColor(.white)
                                .padding(12)
                        }
                        .glassStyle(cornerRadius: 20)
                        
                        // Gemini Diagnosis (Premium AI Card)
                        if let diagnosis = detail.geminiDiagnosis {
                            VStack(alignment: .leading, spacing: 10) {
                                HStack {
                                    Image(systemName: "sparkles")
                                        .foregroundColor(.purple)
                                    Text("AI Diagnosis")
                                        .font(.headline)
                                        .foregroundColor(.purple)
                                    Spacer()
                                    Text("Gemini 2.0")
                                        .font(.caption2.monospaced())
                                        .padding(.horizontal, 6)
                                        .padding(.vertical, 2)
                                        .background(Color.purple.opacity(0.1))
                                        .cornerRadius(4)
                                }
                                
                                Text(diagnosis)
                                    .font(.system(.subheadline, design: .rounded))
                                    .italic()
                                    .foregroundColor(.primary.opacity(0.8))
                            }
                            .padding()
                            .glassStyle(cornerRadius: 16, fill: .purple.opacity(0.05))
                        }
                        
                        // Description & Info Card
                        VStack(alignment: .leading, spacing: 16) {
                            HStack {
                                Text(detail.timeAgo)
                                    .font(.subheadline.bold())
                                    .foregroundColor(.secondary)
                                Spacer()
                                Label(detail.distanceText, systemImage: "location.fill")
                                    .font(.subheadline)
                                    .foregroundColor(.blue)
                            }
                            
                            Text(detail.description ?? "No description provided.")
                                .font(.body)
                                .lineSpacing(4)
                            
                            Divider()
                            
                            if let landmark = detail.landmarkHint {
                                Label {
                                    Text(landmark)
                                        .font(.subheadline)
                                } icon: {
                                    Image(systemName: "mappin.and.ellipse")
                                        .foregroundColor(.red)
                                }
                            }
                        }
                        .padding()
                        .glassStyle(cornerRadius: 20)
                        
                        // Action Buttons
                        VStack(spacing: 12) {
                            if detail.claimState == .available {
                                Button(action: {
                                    UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                                    core.update(.claimRequested(caseId: detail.id))
                                }) {
                                    Text("Claim This Rescue")
                                        .bold()
                                        .frame(maxWidth: .infinity)
                                        .padding()
                                        .background(
                                            LinearGradient(colors: [.green, .emerald], startPoint: .leading, endPoint: .trailing)
                                        )
                                        .foregroundColor(.white)
                                        .clipShape(Capsule())
                                        .shadow(color: .green.opacity(0.3), radius: 10, x: 0, y: 5)
                                }
                            } else if detail.claimState == .claimedByMe {
                                VStack(spacing: 16) {
                                    HStack {
                                        Image(systemName: "checkmark.seal.fill")
                                        Text("You claimed this rescue")
                                    }
                                    .font(.headline)
                                    .foregroundColor(.green)
                                    
                                    LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 12) {
                                        ForEach(detail.availableTransitions, id: \.self) { transition in
                                            Button(action: {
                                                UIImpactFeedbackGenerator(style: .light).impactOccurred()
                                                core.update(.transitionRequested(caseId: detail.id, next: transition))
                                            }) {
                                                Text(titleForTransition(transition))
                                                    .font(.subheadline.bold())
                                                    .frame(maxWidth: .infinity)
                                                    .padding()
                                                    .background(colorForTransition(transition))
                                                    .foregroundColor(.white)
                                                    .cornerRadius(12)
                                            }
                                        }
                                    }
                                }
                                .padding()
                                .glassStyle(cornerRadius: 20, fill: .green.opacity(0.05))
                            } else if detail.claimState == .claimedByOther {
                                Label("Claimed by another volunteer", systemImage: "person.2.fill")
                                    .font(.headline)
                                    .foregroundColor(.secondary)
                                    .frame(maxWidth: .infinity)
                                    .padding()
                                    .glassStyle(cornerRadius: 16)
                            }
                        }
                    }
                    .padding(20)
                }
            }
    }
    
    func titleForTransition(_ t: String) -> String {
        switch t {
        case "en_route": return "En Route"
        case "arrived": return "Arrived"
        case "resolved": return "Resolved"
        case "unreachable": return "Unreachable"
        case "cancel": return "Cancel"
        default: return t.capitalized
        }
    }
    
    func colorForTransition(_ t: String) -> Color {
        switch t {
        case "en_route", "arrived": return .blue
        case "resolved": return .green
        case "unreachable", "cancel": return .red
        default: return .gray
        }
    }
}
