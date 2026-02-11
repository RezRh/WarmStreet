import SwiftUI
import Shared
// import MapLibre check if available in project

struct PinDropView: View {
    @EnvironmentObject var core: Core
    @State private var centerCoordinate = (lat: 0.0, lng: 0.0) // Stub
    
    var body: some View {
        ZStack {
            // MapView placeholder - implementing full MapLibre UIViewRepresentable requires the library
            Color.gray.opacity(0.2)
                .overlay(Text("MapLibre Map Here"))
            
            Image(systemName: "mappin")
                .resizable()
                .frame(width: 30, height: 30)
                .foregroundColor(.red)
                .offset(y: -15) // Pin tip at center
            
            VStack {
                Spacer()
                Button(action: {
                    core.update(.locationPinDropped(lat: centerCoordinate.lat, lng: centerCoordinate.lng))
                }) {
                    Text("Confirm Location")
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(12)
                }
                .padding()
            }
        }
    }
}
