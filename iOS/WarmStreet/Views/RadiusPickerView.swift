import SwiftUI
import Shared

struct RadiusPickerView: View {
    @EnvironmentObject var core: Core
    @State private var selectedRadius: UInt32 = 5000
    
    let options: [UInt32] = [2000, 5000, 10000, 20000, 25000]
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Select Alert Radius")
                .font(.title2)
                .fontWeight(.bold)
                .padding(.top)
            
            // Map Preview Placeholder
            Circle()
                .stroke(Color.blue.opacity(0.5), lineWidth: 2)
                .background(Circle().fill(Color.blue.opacity(0.1)))
                .frame(width: 200, height: 200)
                .overlay(Text("Map Preview"))
            
            Text("You will receive alerts for cases within \(selectedRadius / 1000) km of your location.")
                .multilineTextAlignment(.center)
                .padding()
            
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 80))], spacing: 10) {
                ForEach(options, id: \.self) { radius in
                    RadiusButton(meters: radius, isSelected: selectedRadius == radius) {
                        selectedRadius = radius
                    }
                }
            }
            .padding()
            
            Spacer()
            
            Button(action: {
                core.update(.radiusSelected(meters: selectedRadius))
            }) {
                Text("Continue")
                    .fontWeight(.semibold)
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
