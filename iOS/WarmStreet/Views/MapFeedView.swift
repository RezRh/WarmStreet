import SwiftUI
import Shared
// import MapLibre // Assuming available

struct MapFeedView: View {
    @EnvironmentObject var core: Core
    
    var body: some View {
        ZStack {
            // Placeholder for MapLibre
            // In a real app: MapLibreView(center: $core.mapCenter, pins: core.pins)
            Color.gray.opacity(0.1)
                .edgesIgnoringSafeArea(.top)
            
            if let pins = getPins() {
                ForEach(pins, id: \.id) { pin in
                    // Simple simulated markers
                    // In real MapLibre, these are annotations
                    if pin.lat != 0 && pin.lng != 0 {
                        Circle()
                            .fill(colorForStatus(pin.status))
                            .frame(width: 20, height: 20)
                            .overlay(Circle().stroke(Color.white, lineWidth: 2))
                            // Very rough positioning simulation (useless without real map projection)
                            // Just meant to show logic structure
                            .onTapGesture {
                                core.update(.caseMarkerTapped(caseId: pin.id))
                            }
                    }
                }
            }
            
            VStack {
                Spacer()
                Text("Map View (Stub)")
                    .padding()
                    .background(Color.white.opacity(0.8))
                    .cornerRadius(8)
                    .padding(.bottom, 50)
            }
        }
    }
    
    func getPins() -> [CasePin]? {
        if case let .ready(_, pins, _, _, _, _, _, _) = core.view.state {
            return pins
        }
        return nil
    }
    
    func colorForStatus(_ status: String) -> Color {
        switch status {
        case "pending": return .red
        case "claimed": return .orange
        case "en_route": return .yellow
        case "arrived": return .blue
        case "synced": return .green // local synced state
        default: return .gray
        }
    }
}
