import SwiftUI
import Shared
import CoreLocation

struct LocationPermissionView: View {
    @EnvironmentObject var core: Core
    @State private var locationManager = CLLocationManager()
    
    var body: some View {
        VStack(spacing: 20) {
            Spacer()
            
            Image(systemName: "location.circle.fill")
                .resizable()
                .frame(width: 80, height: 80)
                .foregroundColor(.blue)
            
            Text("Enable Location")
                .font(.title2)
                .fontWeight(.bold)
            
            Text("We use your location to find nearby rescue cases and verify your area of operation.")
                .multilineTextAlignment(.center)
                .foregroundColor(.gray)
                .padding()
            
            Spacer()
            
            Button(action: {
                core.locationHandler.requestPermission()
            }) {
                Text("Use My Current Location")
                    .fontWeight(.semibold)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(12)
            }
            
            Button(action: {
                // Navigate to Pin Drop - how via state?
                // Shared Core drives state. 
                // Pin Drop is an alternative way to provide location data.
                // We likely need a local state navigation or core event "RequestManualLocation".
                // If core doesn't support 'PinDropRequested' event, we handle UI navigation locally
                // but ultimately send 'LocationPinDropped' event.
                // For now, let's assume this view presents PinDropView locally via sheet or NavigationLink
            }) {
                Text("Drop a Pin Instead")
                    .foregroundColor(.blue)
            }
            .padding(.bottom)
        }
        .padding()
    }
}
